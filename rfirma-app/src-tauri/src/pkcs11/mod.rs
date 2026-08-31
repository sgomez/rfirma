//! La capa que habla con el token PKCS#11: la **única** parte de rfirma que
//! toca la clave privada.
//!
//! Hace tres cosas y ninguna más: [`list_certificates`] enumera lo que hay en el
//! token, [`sign`] delega en él la firma de un bloque de bytes, y
//! [`error`](self::error) traduce sus códigos a situaciones que una persona
//! entienda.
//!
//! Es la **fase 2** de la firma trifásica (ID-14, ADR-0001): Java hace la
//! prefirma y la postfirma, y la clave privada nunca sale del token ni llega al
//! isolate de GraalVM. Aquí no hay PDF, ni CAdES, ni orquestación de fases: eso
//! es de otros módulos.
//!
//! # El mecanismo es contraintuitivo
//!
//! Se firma con `CKM_SHA256_RSA_PKCS` y se le pasan los bytes **sin hashear**:
//! hashea él, construye el `DigestInfo` y aplica el relleno (ID-16). Firmar un
//! hash con `CKM_RSA_PKCS` produce una firma matemáticamente válida que ningún
//! validador CAdES/PAdES reconoce, y el PDF *parece* firmado. La razón completa,
//! con las citas del cliente original, está en
//! `docs/research/pkcs11-mecanismo-firma.md`.

pub mod certificate;
pub mod error;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// El mecanismo del ID-16, en un solo sitio para que cambiarlo sea un cambio
/// visible y no un descuido en una llamada perdida.
const SIGNING_MECHANISM: Mechanism<'static> = Mechanism::Sha256RsaPkcs;

/// Los certificados que hay en cualquier token del módulo, sin iniciar sesión.
///
/// No pide el PIN a propósito: los certificados son objetos públicos, y su
/// estado —caducado, en vigor— tiene que poder decidirse **antes** de que a
/// nadie se le pida nada.
pub fn list_certificates(module: &Path) -> Result<Vec<TokenCertificate>, TokenError> {
    let context = context(module)?;
    let mut found = Vec::new();

    for slot in usable_slots(&context)? {
        let token_label = context.get_token_info(slot)?.label().trim().to_owned();
        let session = context.open_ro_session(slot)?;

        for object in session.find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])? {
            let attributes =
                session.get_attributes(object, &[AttributeType::Label, AttributeType::Value])?;

            let mut label = None;
            let mut der = None;
            for attribute in attributes {
                match attribute {
                    Attribute::Label(bytes) => {
                        label = Some(String::from_utf8_lossy(&bytes).trim().to_owned())
                    }
                    Attribute::Value(bytes) => der = Some(bytes),
                    _ => {}
                }
            }

            // Un certificado sin CKA_LABEL no se puede reencontrar por etiqueta,
            // que es lo único que el ADR-0010 deja persistir, así que no se
            // ofrece: enseñarlo sería prometer algo que no podemos cumplir.
            if let (Some(label), Some(der)) = (label, der) {
                if !label.is_empty() {
                    found.push(TokenCertificate::new(
                        CertificateRef::new(module, &token_label, label),
                        der,
                    ));
                }
            }
        }
    }

    Ok(found)
}

/// Firma `data` con la clave privada que acompaña al certificado referenciado.
///
/// `data` son los bytes **sin hashear** —los DER de `PRE` en el recorrido
/// trifásico— y lo que vuelve es la firma **cruda**, sin codificar: quien la
/// meta en el campo `PK1` es quien la pasa a Base64.
pub fn sign(reference: &CertificateRef, pin: &str, data: &[u8]) -> Result<Vec<u8>, TokenError> {
    with_token_turn(|| sign_holding_the_turn(reference, pin, data))
}

/// El turno del token: **todo** lo que inicie sesión contra él pasa por aquí.
///
/// El estado de sesión iniciada de PKCS#11 es *del token dentro del proceso*,
/// no de la sesión: dos inicios de sesión a la vez se pisan, y el segundo
/// recibe `CKR_USER_ALREADY_LOGGED_IN` en vez de autenticar. Peor: un
/// `logout()` de un lado cierra la sesión autenticada del otro.
///
/// Está público —y oculto de la documentación— porque las pruebas de
/// integración de `tests/pkcs11_token.rs` corren en hilos del mismo proceso y
/// una de ellas inicia sesión a mano, como contraejemplo del mecanismo del
/// ID-16. Sin este turno compartido, ese contraejemplo se cruza con las firmas
/// de verdad y pone el carril rápido rojo de forma intermitente. **No es
/// reentrante**: nunca llames a [`sign`] desde dentro del cierre.
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
    let context = context(reference.module())?;
    let slot = slot_of(&context, reference.token_label())?;
    let session = context.open_ro_session(slot)?;

    match session.login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => {}
        // Ya autenticados contra este token: el PIN no hace falta otra vez.
        //
        // Ojo con lo que eso implica: por esta rama **el PIN que ha escrito la
        // persona no se comprueba**, se firma con la sesión que abrió otro.
        // Dentro de rfirma es inalcanzable —`with_token_turn` serializa y el
        // `logout()` de abajo se ejecuta salga bien o mal—, así que solo llega
        // aquí si **otra biblioteca del mismo proceso** ha iniciado sesión
        // contra este token. Tratarlo como fallo dejaría sin firmar a quien
        // tenga el token compartido; tratarlo como éxito es lo que hacemos, y
        // el precio es ese PIN sin validar. Cuando llegue el diálogo del PIN,
        // esta es la distinción que hay que tener presente.
        Err(Error::Pkcs11(RvError::UserAlreadyLoggedIn, _)) => {}
        Err(other) => return Err(other.into()),
    }

    let signature = private_key(&session, reference.label()).and_then(|key| {
        session
            .sign(&SIGNING_MECHANISM, key, data)
            .map_err(TokenError::from)
    });

    // Se cierra la sesión autenticada en cuanto deja de hacer falta, salga bien
    // o mal: dejarla abierta es dejar el token desbloqueado para todo el proceso.
    let _ = session.logout();

    signature
}

/// Las ranuras con un token **ya inicializado**.
///
/// No vale `get_slots_with_token`: SoftHSM anuncia también su ranura libre, y un
/// lector de tarjetas anuncia la suya vacía. Abrir sesión contra ellas devuelve
/// `CKR_TOKEN_NOT_RECOGNIZED` y convertiría «hay un lector sin tarjeta» en un
/// fallo del programa.
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

fn private_key(
    session: &Session,
    label: &str,
) -> Result<cryptoki::object::ObjectHandle, TokenError> {
    session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(label.as_bytes().to_vec()),
        ])?
        .into_iter()
        .next()
        .ok_or_else(|| {
            TokenError::new(
                Situation::CertificateNotFound,
                format!("el token no tiene ninguna clave privada etiquetada {label}"),
            )
        })
}

/// El contexto de un módulo, cacheado para todo el proceso.
///
/// No es una optimización: `C_Initialize` **solo se puede llamar una vez por
/// proceso y módulo**, y la segunda devuelve `CKR_CRYPTOKI_ALREADY_INITIALIZED`.
/// Como el `.so` lo comparte todo el proceso vía `dlopen`, abrir un contexto por
/// operación rompería en cuanto haya dos.
fn context(module: &Path) -> Result<Arc<Pkcs11>, TokenError> {
    static MODULES: OnceLock<Mutex<HashMap<PathBuf, Arc<Pkcs11>>>> = OnceLock::new();

    let mut loaded = MODULES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        // Un envenenamiento aquí significa que otro hilo entró en pánico con el
        // candado cogido. El mapa sigue siendo válido —solo guarda contextos ya
        // inicializados—, así que perder el proceso por eso sería peor.
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    if let Some(context) = loaded.get(module) {
        return Ok(Arc::clone(context));
    }

    let context = Pkcs11::new(module)?;
    match context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK)) {
        Ok(()) => {}
        // `C_Initialize` es por proceso, no por handle: si otra biblioteca del
        // mismo proceso ya cargó este módulo, el nuestro está inicializado y
        // tratarlo como fallo dejaría la aplicación sin token por buenas noticias.
        Err(Error::Pkcs11(RvError::CryptokiAlreadyInitialized, _)) => {}
        Err(other) => return Err(other.into()),
    }
    let context = Arc::new(context);
    loaded.insert(module.to_path_buf(), Arc::clone(&context));

    Ok(context)
}
