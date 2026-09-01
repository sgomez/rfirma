//! Sonda del sondeo #117: qué dicen `CK_TOKEN_INFO.flags` y `C_Login` en cada
//! almacén que rfirma lista, y si `C_Sign` funciona sin iniciar sesión.
//!
//! Uso: `token-flags-probe <módulo.so> [init_args] -- <pin>...`
//!
//! Cada invocación abre **un** almacén: `C_Initialize` es por proceso y
//! módulo, y los perfiles NSS se distinguen solo por los init args, así que
//! sondear dos perfiles en el mismo proceso obligaría a `C_Finalize` entre
//! medias, que es lo que hace rfirma pero no lo que se quiere medir aquí.
//!
//! Para cada PIN de la lista se abre una sesión nueva y se prueba
//! `C_Login(User, pin)` seguido de un `C_Sign`. Las palabras `EMPTY` y `NONE`
//! significan `""` (puntero válido, longitud 0) y `NULL` (sin PIN).
//! Antes de los PIN se prueba `C_Sign` **sin** `C_Login`.

use std::env;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::Path;
use std::ptr::NonNull;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let split = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let (store, pins) = args.split_at(split);
    let pins: Vec<&String> = pins.iter().skip(1).collect();
    let module = store.first().expect("falta la ruta del módulo");
    let init_args = store.get(1).map(String::as_str);

    println!("## módulo {module}");
    if let Some(init_args) = init_args {
        println!("   init args: {init_args}");
    }

    let context = Pkcs11::new(Path::new(module)).expect("no carga el módulo");
    let flags = CInitializeFlags::OS_LOCKING_OK;
    let reserved = init_args.map(|a| CString::new(a).unwrap());
    let arguments = match &reserved {
        Some(reserved) => unsafe {
            CInitializeArgs::new_with_reserved(
                flags,
                NonNull::new(reserved.as_ptr() as *mut c_void).unwrap(),
            )
        },
        None => CInitializeArgs::new(flags),
    };
    context.initialize(arguments).expect("C_Initialize falló");

    let slots = context
        .get_slots_with_initialized_token()
        .expect("get_slots_with_initialized_token");
    println!("   ranuras con token inicializado: {}", slots.len());

    for slot in slots {
        probe_slot(&context, slot, &pins);
    }
}

fn probe_slot(context: &Pkcs11, slot: Slot, pins: &[&String]) {
    let info = context.get_token_info(slot).expect("get_token_info");
    println!("\n### ranura {} — token «{}»", slot.id(), info.label().trim());
    println!("   CKF_LOGIN_REQUIRED               = {}", info.login_required());
    println!("   CKF_USER_PIN_INITIALIZED         = {}", info.user_pin_initialized());
    println!(
        "   CKF_PROTECTED_AUTHENTICATION_PATH = {}",
        info.protected_authentication_path()
    );
    println!("   CKF_TOKEN_INITIALIZED            = {}", info.token_initialized());
    println!(
        "   PIN min/max                      = {}/{}",
        info.min_pin_length(),
        info.max_pin_length()
    );

    // Sin C_Login: ¿se ven claves privadas y se puede firmar?
    {
        let session = context.open_ro_session(slot).expect("open_ro_session");
        println!("   sin C_Login: {}", sign_report(&session));
    }

    for pin in pins {
        let session = context.open_ro_session(slot).expect("open_ro_session");
        let (label, result) = match pin.as_str() {
            "NONE" => ("C_Login(User, NULL)".to_owned(), session.login(UserType::User, None)),
            "EMPTY" => (
                "C_Login(User, \"\")".to_owned(),
                session.login(UserType::User, Some(&AuthPin::new(String::new().into()))),
            ),
            other => (
                format!("C_Login(User, {other:?})"),
                session.login(UserType::User, Some(&AuthPin::new(other.to_owned().into()))),
            ),
        };
        let login = match &result {
            Ok(()) => "CKR_OK".to_owned(),
            Err(e) => e.to_string(),
        };
        println!("   {label:<24} -> {login}; después {}", sign_report(&session));
        let _ = session.logout();
    }
}

/// Cuántas claves privadas se ven en la sesión y qué devuelve `C_Sign` con
/// la primera, con el mecanismo que usa rfirma (`CKM_SHA256_RSA_PKCS`).
fn sign_report(session: &Session) -> String {
    let keys = match session.find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)]) {
        Ok(keys) => keys,
        Err(e) => return format!("find_objects(PRIVATE_KEY) -> {e}"),
    };
    let Some(key) = keys.first() else {
        return "0 claves privadas visibles, nada que firmar".to_owned();
    };
    let label = session
        .get_attributes(*key, &[AttributeType::Label])
        .ok()
        .and_then(|a| a.into_iter().next())
        .and_then(|a| match a {
            Attribute::Label(b) => Some(String::from_utf8_lossy(&b).into_owned()),
            _ => None,
        })
        .unwrap_or_default();
    match session.sign(&Mechanism::Sha256RsaPkcs, *key, b"hola") {
        Ok(sig) => format!(
            "{} claves privadas visibles; C_Sign con «{label}» -> CKR_OK ({} bytes)",
            keys.len(),
            sig.len()
        ),
        Err(e) => format!(
            "{} claves privadas visibles; C_Sign con «{label}» -> {e}",
            keys.len()
        ),
    }
}
