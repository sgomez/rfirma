//! Capa de acceso a tokens criptográficos y firma nativa PKCS#11 (ADR-0001).

pub mod certificate;
pub mod error;
pub mod nss;
pub mod secret;
pub mod stores;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex, OnceLock};

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;

pub use certificate::{CertificateRef, CertificateStatus, TokenCertificate};
pub use error::{Situation, TokenError};
pub use secret::{SecretOnTheReaderKeypad, StoreSecret};
pub use stores::{Store, StoreClass};

/// Mecanismo de firma digital utilizado en las operaciones PKCS#11.
const SIGNING_MECHANISM: Mechanism<'static> = Mechanism::Sha256RsaPkcs;

/// Lista los certificados presentes en todos los almacenes indicados.
pub fn list_certificates_across(stores: &[Store]) -> Result<Vec<TokenCertificate>, TokenError> {
    if stores.is_empty() {
        return Err(TokenError::new(
            Situation::ModuleNotFound,
            "no hay ningun modulo PKCS#11 donde buscar certificados",
        ));
    }

    let mut found = Vec::new();
    let mut any_loaded = false;
    let mut refused: Option<TokenError> = None;

    for store in stores {
        match list_certificates(store) {
            Ok(certificates) => {
                any_loaded = true;
                found.extend(certificates);
            }
            Err(error) => refused = refused.or(Some(error)),
        }
    }

    match refused {
        Some(error) if !any_loaded => Err(error),
        _ => Ok(found),
    }
}

/// Lista los certificados firmables disponibles en el almacén indicado.
pub fn list_certificates(store: impl Into<Store>) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    with_token_turn(|| list_holding_the_turn(&store))
}

fn list_holding_the_turn(store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
    the_store_is_really_there(store)?;
    let context = context(store)?;
    let mut found = Vec::new();

    for slot in usable_slots(&context)? {
        let info = context.get_token_info(slot)?;
        let token_label = info.label().trim().to_owned();
        let session = context.open_ro_session(slot)?;

        let logged_in = log_in_before_listing(&session, &info);
        found.extend(signable_certificates(
            &session,
            store,
            &token_label,
            logged_in,
        )?);
        if logged_in {
            let _ = session.logout();
        }
    }

    Ok(found)
}

/// Inicia sesión automáticamente antes de listar si la ranura lo requiere.
fn log_in_before_listing(session: &Session, info: &cryptoki::slot::TokenInfo) -> bool {
    if !should_attempt_blind_login(info.login_required(), info.protected_authentication_path()) {
        return false;
    }

    match session.login(UserType::User, None) {
        Ok(()) => true,
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => false,
        Err(_) => false,
    }
}

/// Determina si procede intentar un inicio de sesión ciego para listar.
fn should_attempt_blind_login(login_required: bool, protected_authentication_path: bool) -> bool {
    login_required && !protected_authentication_path
}

/// Filtra certificados de una ranura conservando aquellos con clave privada emparejada.
fn signable_certificates(
    session: &Session,
    store: &Store,
    token_label: &str,
    logged_in: bool,
) -> Result<Vec<TokenCertificate>, TokenError> {
    if store.class() == StoreClass::Card && !logged_in {
        return all_certificates_in_session(session, store, token_label);
    }

    let visible_private_keys = private_key_ids(session)?;

    let mut found = Vec::new();
    for certificate in all_certificates_in_session(session, store, token_label)? {
        if certificate
            .reference()
            .cka_id()
            .is_some_and(|cka_id| visible_private_keys.contains(cka_id))
        {
            found.push(certificate);
        }
    }

    Ok(found)
}

/// Los `CKA_ID` de las claves privadas visibles en la sesión.
fn private_key_ids(session: &Session) -> Result<HashSet<Vec<u8>>, TokenError> {
    let mut ids = HashSet::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::PRIVATE_KEY)])? {
        let Ok(attributes) = session.get_attributes(object, &[AttributeType::Id]) else {
            continue;
        };

        for attribute in attributes {
            if let Attribute::Id(bytes) = attribute {
                if !bytes.is_empty() {
                    ids.insert(bytes);
                }
            }
        }
    }

    Ok(ids)
}

/// Certificados de una ranura abierta sin aplicar filtro de clave privada.
fn all_certificates_in_session(
    session: &Session,
    store: &Store,
    token_label: &str,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let mut found = Vec::new();

    for object in session.find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])? {
        let attributes = session.get_attributes(
            object,
            &[
                AttributeType::Label,
                AttributeType::Value,
                AttributeType::Id,
            ],
        )?;

        let mut label = None;
        let mut der = None;
        let mut cka_id = None;
        for attribute in attributes {
            match attribute {
                Attribute::Label(bytes) => {
                    label = Some(String::from_utf8_lossy(&bytes).trim().to_owned())
                }
                Attribute::Value(bytes) => der = Some(bytes),
                Attribute::Id(bytes) if !bytes.is_empty() => cka_id = Some(bytes),
                _ => {}
            }
        }

        if let (Some(label), Some(der)) = (label, der) {
            if !label.is_empty() {
                found.push(TokenCertificate::new(
                    CertificateRef::new(store, token_label, label, cka_id),
                    der,
                ));
            }
        }
    }

    Ok(found)
}

/// Listado de certificados sin filtrar por clave privada para pruebas.
#[doc(hidden)]
pub fn list_certificates_unfiltered_for_test(
    store: impl Into<Store>,
) -> Result<Vec<TokenCertificate>, TokenError> {
    let store = store.into();
    with_token_turn(|| {
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let mut found = Vec::new();

        for slot in usable_slots(&context)? {
            let token_label = context.get_token_info(slot)?.label().trim().to_owned();
            let session = context.open_ro_session(slot)?;
            found.extend(all_certificates_in_session(&session, &store, &token_label)?);
        }

        Ok(found)
    })
}

/// Comprueba si existe la base de datos NSS antes de inicializar el módulo.
fn the_store_is_really_there(store: &Store) -> Result<(), TokenError> {
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

/// Cómo hay que pedirle el secreto al almacén leyendo las banderas de su ranura.
pub fn store_secret(reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
    with_token_turn(|| {
        let store = reference.store();
        the_store_is_really_there(&store)?;
        let context = context(&store)?;
        let slot = slot_of(&context, reference.token_label())?;
        let info = context.get_token_info(slot)?;
        Ok(StoreSecret::of_token(
            info.login_required(),
            info.protected_authentication_path(),
        ))
    })
}

/// Firma `data` con la clave privada que acompaña al certificado referenciado.
pub fn sign(reference: &CertificateRef, pin: &str, data: &[u8]) -> Result<Vec<u8>, TokenError> {
    with_token_turn(|| sign_holding_the_turn(reference, pin, data))
}

/// Serializa operaciones contra el token en el proceso para evitar colisiones de sesión.
#[doc(hidden)]
pub fn with_token_turn<T>(operation: impl FnOnce() -> T) -> T {
    static TURN: Mutex<()> = Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}

fn sign_holding_the_turn(
    reference: &CertificateRef,
    pin: &str,
    data: &[u8],
) -> Result<Vec<u8>, TokenError> {
    let store = reference.store();
    the_store_is_really_there(&store)?;
    let context = context(&store)?;
    let slot = slot_of(&context, reference.token_label())?;
    let session = context.open_ro_session(slot)?;

    match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => {}
        // Si otra biblioteca del proceso ya autenticó el token, se reutiliza la sesión.
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => {}
        Err(other) => return Err(other.into()),
    }

    let signature = private_key(&session, reference).and_then(|key| {
        session
            .sign(&SIGNING_MECHANISM, key, data)
            .map_err(TokenError::from)
    });

    let _ = session.logout();

    signature
}

/// Las ranuras con un token ya inicializado.
fn usable_slots(context: &Pkcs11) -> Result<Vec<Slot>, TokenError> {
    Ok(context.get_slots_with_initialized_token()?)
}

fn slot_of(context: &Pkcs11, token_label: &str) -> Result<Slot, TokenError> {
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
fn private_key(
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
fn context(store: &Store) -> Result<Arc<Pkcs11>, TokenError> {
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

#[cfg(test)]
mod log_in_before_listing_tests {
    use super::should_attempt_blind_login;

    #[test]
    fn skips_a_slot_that_does_not_require_login() {
        assert!(!should_attempt_blind_login(false, false));
    }

    #[test]
    fn attempts_a_blind_login_on_a_slot_without_a_reader_keypad() {
        assert!(should_attempt_blind_login(true, false));
    }

    #[test]
    fn skips_a_slot_with_a_reader_keypad_even_if_login_is_required() {
        assert!(!should_attempt_blind_login(true, true));
    }
}
