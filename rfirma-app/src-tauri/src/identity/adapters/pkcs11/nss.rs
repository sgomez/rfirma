//! Importación de ficheros PKCS#12 en almacenes NSS propios (ADR-0001).

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void, CString};
use std::path::Path;
use std::sync::OnceLock;

use libloading::Library;

use crate::identity::ports::NssHost;

use super::stores::present_among;
use crate::identity::domain::error::{Situation, TokenError};

/// Rutas candidatas para localizar la biblioteca `libnss3.so`.
pub const CANDIDATE_NSS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libnss3.so",
    "/usr/lib64/libnss3.so",
    "/usr/lib/libnss3.so",
];

/// Rutas candidatas para localizar la biblioteca `libsmime3.so`.
pub const CANDIDATE_SMIME: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libsmime3.so",
    "/usr/lib64/libsmime3.so",
    "/usr/lib/libsmime3.so",
];

const SEC_SUCCESS: c_int = 0;
const PR_TRUE: c_int = 1;
const SI_BUFFER: c_uint = 0;

fn module_spec(directory: &Path) -> String {
    format!(
        "configDir='sql:{}' certPrefix='' keyPrefix='' \
         tokenDescription='rfirma' flags=readWrite",
        directory.display()
    )
}

#[repr(C)]
struct SecItem {
    kind: c_uint,
    data: *mut c_uchar,
    len: c_uint,
}

struct Password {
    bytes: Vec<u8>,
}

impl Password {
    fn item(&mut self) -> SecItem {
        SecItem {
            kind: SI_BUFFER,
            data: self.bytes.as_mut_ptr(),
            len: self.bytes.len() as c_uint,
        }
    }
}

fn bmp_string(password: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(password.len() * 2 + 2);
    for unit in password.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

extern "C" fn no_password(
    _slot: *mut c_void,
    _retry: c_int,
    _argument: *mut c_void,
) -> *mut c_char {
    std::ptr::null_mut()
}

extern "C" fn keep_the_nickname(
    _old: *mut SecItem,
    cancel: *mut c_int,
    _argument: *mut c_void,
) -> *mut SecItem {
    if !cancel.is_null() {
        // SAFETY: NSS pasa aquí un `PRBool` suyo, vivo durante la llamada.
        unsafe { *cancel = 0 };
    }
    std::ptr::null_mut()
}

/// Detalle del error cuando la biblioteca `libnss3.so` no está disponible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NssUnavailable {
    detail: String,
}

impl NssUnavailable {
    /// Construye un error con el detalle correspondiente.
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Detalle del motivo por el que la biblioteca no está disponible.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for NssUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for NssUnavailable {}

/// Adaptador de [`NssHost`] sobre la biblioteca `libnss3.so` del sistema y el turno del token.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealNssHost;

impl NssHost for RealNssHost {
    fn library(&self) -> Result<&'static Library, NssUnavailable> {
        nss_library()
    }

    fn with_token_turn<T>(&self, work: impl FnOnce() -> T) -> T {
        super::with_token_turn(work)
    }
}

impl<H: NssHost> NssHost for &H {
    fn library(&self) -> Result<&'static Library, NssUnavailable> {
        (*self).library()
    }

    fn with_token_turn<T>(&self, work: impl FnOnce() -> T) -> T {
        (*self).with_token_turn(work)
    }
}

static NSS_LIBRARY: OnceLock<Result<Library, String>> = OnceLock::new();
static SMIME_LIBRARY: OnceLock<Result<Library, String>> = OnceLock::new();

/// Carga la biblioteca `libnss3.so` compartida del sistema.
pub fn nss_library() -> Result<&'static Library, NssUnavailable> {
    let loaded = NSS_LIBRARY.get_or_init(|| {
        let path = present_among(CANDIDATE_NSS, |path| path.is_file())
            .into_iter()
            .next()
            .ok_or_else(|| "no esta libnss3.so en ninguna de las rutas conocidas".to_owned())?;
        // SAFETY: la ruta sale de la lista cerrada de candidatos de este módulo, no
        // del entorno, y la biblioteca es la de NSS del sistema.
        unsafe { Library::new(&path) }.map_err(|error| format!("{}: {error}", path.display()))
    });
    loaded
        .as_ref()
        .map_err(|detail| NssUnavailable::new(detail.clone()))
}

fn smime_library() -> Result<&'static Library, TokenError> {
    let loaded = SMIME_LIBRARY.get_or_init(|| first_present(CANDIDATE_SMIME, "libsmime3.so"));
    loaded
        .as_ref()
        .map_err(|detail| TokenError::new(Situation::ModuleNotFound, detail.clone()))
}

fn first_present(candidates: &[&str], name: &str) -> Result<Library, String> {
    let path = present_among(candidates, |path| path.is_file())
        .into_iter()
        .next()
        .ok_or_else(|| format!("no esta {name} en ninguna de las rutas conocidas"))?;
    // SAFETY: la ruta sale de la lista cerrada de candidatos de este módulo, no
    // del entorno, y las dos bibliotecas son las de NSS del sistema.
    unsafe { Library::new(&path) }.map_err(|error| format!("{}: {error}", path.display()))
}

fn symbol<T: Copy>(library: &'static Library, name: &[u8]) -> Result<T, TokenError> {
    // SAFETY: cada tipo `T` de este módulo es la firma declarada en la cabecera
    // pública de NSS para ese símbolo, y las bibliotecas viven hasta que muere
    // el proceso.
    unsafe { library.get::<T>(name) }
        .map(|symbol| *symbol)
        .map_err(|error| {
            TokenError::new(
                Situation::ModuleNotFound,
                format!(
                    "NSS no exporta «{}»: {error}",
                    String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
                ),
            )
        })
}

fn failed(step: &str) -> TokenError {
    TokenError::new(
        Situation::Pkcs12Unreadable,
        format!("NSS ha fallado en {step}"),
    )
}

/// Importa un fichero PKCS#12 en el almacén NSS indicado.
pub fn import_pkcs12(directory: &Path, pkcs12: &[u8], password: &str) -> Result<(), TokenError> {
    let nss = nss_library()
        .map_err(|err| TokenError::new(Situation::ModuleNotFound, err.detail().to_owned()))?;
    let smime = smime_library()?;

    type NoDbInit = extern "C" fn(*const c_char) -> c_int;
    type Shutdown = extern "C" fn() -> c_int;
    type OpenUserDb = extern "C" fn(*const c_char) -> *mut c_void;
    type CloseUserDb = extern "C" fn(*mut c_void) -> c_int;
    type NeedUserInit = extern "C" fn(*mut c_void) -> c_int;
    type InitPin = extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
    type Authenticate = extern "C" fn(*mut c_void, c_int, *mut c_void) -> c_int;
    type FreeSlot = extern "C" fn(*mut c_void);
    type SetPasswordFunc =
        extern "C" fn(extern "C" fn(*mut c_void, c_int, *mut c_void) -> *mut c_char);
    type DecoderStart = extern "C" fn(
        *mut SecItem,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) -> *mut c_void;
    type DecoderUpdate = extern "C" fn(*mut c_void, *const c_uchar, c_ulong) -> c_int;
    type DecoderStep = extern "C" fn(*mut c_void) -> c_int;
    type DecoderValidate = extern "C" fn(
        *mut c_void,
        extern "C" fn(*mut SecItem, *mut c_int, *mut c_void) -> *mut SecItem,
    ) -> c_int;
    type DecoderFinish = extern "C" fn(*mut c_void);

    let nss_no_db_init: NoDbInit = symbol(nss, b"NSS_NoDB_Init\0")?;
    let nss_shutdown: Shutdown = symbol(nss, b"NSS_Shutdown\0")?;
    let open_user_db: OpenUserDb = symbol(nss, b"SECMOD_OpenUserDB\0")?;
    let close_user_db: CloseUserDb = symbol(nss, b"SECMOD_CloseUserDB\0")?;
    let need_user_init: NeedUserInit = symbol(nss, b"PK11_NeedUserInit\0")?;
    let init_pin: InitPin = symbol(nss, b"PK11_InitPin\0")?;
    let authenticate: Authenticate = symbol(nss, b"PK11_Authenticate\0")?;
    let free_slot: FreeSlot = symbol(nss, b"PK11_FreeSlot\0")?;
    let set_password_func: SetPasswordFunc = symbol(nss, b"PK11_SetPasswordFunc\0")?;
    let decoder_start: DecoderStart = symbol(smime, b"SEC_PKCS12DecoderStart\0")?;
    let decoder_update: DecoderUpdate = symbol(smime, b"SEC_PKCS12DecoderUpdate\0")?;
    let decoder_verify: DecoderStep = symbol(smime, b"SEC_PKCS12DecoderVerify\0")?;
    let decoder_validate: DecoderValidate = symbol(smime, b"SEC_PKCS12DecoderValidateBags\0")?;
    let decoder_import: DecoderStep = symbol(smime, b"SEC_PKCS12DecoderImportBags\0")?;
    let decoder_finish: DecoderFinish = symbol(smime, b"SEC_PKCS12DecoderFinish\0")?;

    let spec = CString::new(module_spec(directory)).map_err(|_| {
        TokenError::new(
            Situation::ModuleNotFound,
            "la ruta del almacen lleva un cero dentro",
        )
    })?;
    let empty = CString::new("").expect("la cadena vacia no lleva ceros dentro");
    let mut secret = Password {
        bytes: bmp_string(password),
    };

    set_password_func(no_password);

    if nss_no_db_init(std::ptr::null()) != SEC_SUCCESS {
        return Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "NSS no ha podido arrancar sin base de datos: algo del proceso tiene ya \
             inicializado el softoken (¿RFIRMA_PKCS11_MODULE apuntando a \
             libsoftokn3.so?)",
        ));
    }

    let outcome = (|| {
        let slot = open_user_db(spec.as_ptr());
        if slot.is_null() {
            return Err(failed("SECMOD_OpenUserDB"));
        }

        let imported = (|| {
            if need_user_init(slot) == PR_TRUE
                && init_pin(slot, std::ptr::null(), empty.as_ptr()) != SEC_SUCCESS
            {
                return Err(failed("PK11_InitPin"));
            }
            if authenticate(slot, PR_TRUE, std::ptr::null_mut()) != SEC_SUCCESS {
                return Err(failed("PK11_Authenticate"));
            }

            let mut item = secret.item();
            let decoder = decoder_start(
                &mut item,
                slot,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            if decoder.is_null() {
                return Err(failed("SEC_PKCS12DecoderStart"));
            }

            let decoded = (|| {
                if decoder_update(decoder, pkcs12.as_ptr(), pkcs12.len() as c_ulong) != SEC_SUCCESS
                {
                    return Err(failed("SEC_PKCS12DecoderUpdate"));
                }
                if decoder_verify(decoder) != SEC_SUCCESS {
                    return Err(failed("SEC_PKCS12DecoderVerify"));
                }
                if decoder_validate(decoder, keep_the_nickname) != SEC_SUCCESS {
                    return Err(failed("SEC_PKCS12DecoderValidateBags"));
                }
                if decoder_import(decoder) != SEC_SUCCESS {
                    return Err(failed("SEC_PKCS12DecoderImportBags"));
                }
                Ok(())
            })();

            decoder_finish(decoder);
            decoded
        })();

        close_user_db(slot);
        free_slot(slot);
        imported
    })();

    nss_shutdown();
    outcome
}

#[cfg(test)]
mod tests;
