//! Apertura del módulo PKCS#11 y de sus ranuras: contexto, sesión y clave privada.

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};
use cryptoki::object::{Attribute, ObjectClass};
use cryptoki::session::Session;
use cryptoki::slot::Slot;

use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::store::Store;

/// Comprueba si existe la base de datos NSS antes de inicializar el módulo.
pub(super) fn the_store_is_really_there(store: &Store) -> Result<(), TokenError> {
    let Some(init_args) = store.init_args() else {
        return Ok(());
    };

    let profile = configured_directory(init_args);
    if profile.is_some_and(|directory| Path::new(directory).join("cert9.db").is_file()) {
        return Ok(());
    }

    Err(TokenError::new(
        Situation::ModuleNotFound,
        format!(
            "los init args «{init_args}» no llevan a ningun perfil NSS: \
             detras del configdir no hay ningun cert9.db"
        ),
    ))
}

/// El `configdir` de unos init args, ya sin el prefijo `sql:` ni las comillas.
fn configured_directory(init_args: &str) -> Option<&str> {
    let value = init_args.split("configdir=").nth(1)?;
    let value = value.strip_prefix('\'')?;
    let value = value.split('\'').next()?;
    Some(value.strip_prefix("sql:").unwrap_or(value))
}

/// Las ranuras con un token ya inicializado.
pub(super) fn usable_slots(context: &Pkcs11) -> Result<Vec<Slot>, TokenError> {
    Ok(context.get_slots_with_initialized_token()?)
}

pub(super) fn slot_of(context: &Pkcs11, token_label: &str) -> Result<Slot, TokenError> {
    for slot in usable_slots(context)? {
        if context.get_token_info(slot)?.label().trim() == token_label {
            return Ok(slot);
        }
    }
    Err(TokenError::new(
        Situation::TokenAbsent,
        format!("no hay ningun token etiquetado {token_label}"),
    ))
}

/// La clave privada del certificado emparejada por `CKA_ID`.
pub(super) fn private_key(
    session: &Session,
    reference: &CertificateRef,
) -> Result<cryptoki::object::ObjectHandle, TokenError> {
    let label = reference.label();
    let cka_id = reference.cka_id().ok_or_else(|| {
        TokenError::new(
            Situation::CertificateNotFound,
            format!("la referencia a {label} no lleva CKA_ID: vuelve a listar el token"),
        )
    })?;

    session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Id(cka_id.to_vec()),
        ])?
        .into_iter()
        .next()
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el token no tiene ninguna clave privada con el CKA_ID de {label}"),
            )
        })
}

/// El contexto de un módulo, cacheado para todo el proceso.
pub(super) fn context(store: &Store) -> Result<Arc<Pkcs11>, TokenError> {
    static MODULES: OnceLock<Mutex<HashMap<PathBuf, Arc<Pkcs11>>>> = OnceLock::new();

    // Un almacén con init args no se cachea porque C_Initialize es por proceso y módulo.
    if let Some(init_args) = store.init_args() {
        return Ok(Arc::new(initialized(store.path(), Some(init_args))?));
    }

    let mut loaded = MODULES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(context) = loaded.get(store.path()) {
        return Ok(Arc::clone(context));
    }

    let context = Arc::new(initialized(store.path(), None)?);
    loaded.insert(store.path().to_path_buf(), Arc::clone(&context));

    Ok(context)
}

/// Carga el módulo y llama a `C_Initialize`, con init args o sin ellos.
fn initialized(module: &Path, init_args: Option<&str>) -> Result<Pkcs11, TokenError> {
    let context = Pkcs11::new(module)?;
    let flags = CInitializeFlags::OS_LOCKING_OK;

    let reserved = init_args
        .map(|args| CString::new(args).map_err(|_| nul_inside(args)))
        .transpose()?;
    let arguments = match &reserved {
        // SAFETY: el puntero procede de una CString viva durante C_Initialize.
        Some(reserved) => unsafe {
            CInitializeArgs::new_with_reserved(
                flags,
                NonNull::new(reserved.as_ptr() as *mut c_void)
                    .expect("una CString nunca esta en la direccion cero"),
            )
        },
        None => CInitializeArgs::new(flags),
    };

    match context.initialize(arguments) {
        Ok(()) => {}
        // Si otra biblioteca del proceso ya inicializó el módulo, se considera éxito.
        Err(Error::Pkcs11(RvError::CryptokiAlreadyInitialized, _)) => {}
        Err(other) => return Err(other.into()),
    }

    Ok(context)
}

fn nul_inside(init_args: &str) -> TokenError {
    TokenError::new(
        Situation::ModuleNotFound,
        format!("los init args «{init_args}» llevan un cero dentro"),
    )
}
