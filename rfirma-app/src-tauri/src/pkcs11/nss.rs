//! **Cómo entra un `.p12` en un almacén NSS propio**, sin que rfirma descifre
//! nada (ID-192, ID-193).
//!
//! El fichero no se parsea aquí ni en ningún otro sitio de Rust: lo abre el
//! descodificador de PKCS#12 de NSS, que ya está en las tres formas de
//! distribución, y lo que queda detrás es un almacén NSS corriente —`cert9.db`
//! y `key4.db` y nada más— que el resto de [`crate::pkcs11`] abre como abre el
//! perfil de un Firefox. **No es un motor de firma**: `C_Sign` sigue siendo el
//! único sitio que toca la clave (ADR-0001).
//!
//! # Dos cosas que se midieron y no son las que uno supondría
//!
//! Las dos están en `docs/research/p12-en-almacen-nss.md`:
//!
//! - **Los símbolos de PKCS#12 no están en `libnss3.so`**, sino en
//!   `libsmime3.so`. Por eso aquí se cargan dos bibliotecas y no una.
//! - **`NSS_Init` sobre un `configdir` y el `C_Initialize` de `cryptoki` no
//!   pueden convivir** (ID-194). Con softoken ya inicializado, cualquier
//!   `NSS_*_Init` falla; y al revés es peor, porque
//!   [`super::initialized`] se traga a propósito el
//!   `CKR_CRYPTOKI_ALREADY_INITIALIZED` y entonces lista **el almacén de NSS**
//!   creyendo que lista el que le pidieron, sin ningún error. Lo que salva el
//!   camino es que aquí no se inicializa NSS con base de datos:
//!   `NSS_NoDB_Init` más `SECMOD_OpenUserDB` abren el almacén del fichero como
//!   una ranura suelta, y todo ello **dentro del turno del token**
//!   ([`super::with_token_turn`]), que ya garantiza que softoken no está
//!   inicializado, y termina con `SECMOD_CloseUserDB` y `NSS_Shutdown` antes de
//!   soltarlo.
//!
//! # Lo que no hace
//!
//! - **No mete la cadena de CA en el almacén.** Entrarían con el apodo `(NULL)`
//!   —medido en el sondeo— y el listado las descartaría igual, porque solo
//!   enseña certificados con clave privada emparejada (ID-07).
//! - **No reintenta la contraseña con los bytes intercambiados.** El
//!   descodificador quiere un `BMPString` UCS-2 *big endian*, que es lo que
//!   escribe [`bmp_string`]; un `.p12` generado con UCS-2 *little endian* y una
//!   contraseña con acentos necesitaría el reintento que hace `pk12util`. No
//!   hay ningún fichero con el que probarlo, y código sin prueba que lo mire es
//!   código que no se sabe si funciona.

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void, CString};
use std::path::Path;
use std::sync::OnceLock;

use libloading::Library;

use super::error::{Situation, TokenError};
use super::stores::present_among;

/// Dónde puede estar `libnss3.so`, que es quien inicializa NSS y abre la ranura.
///
/// Se declaran por ruta absoluta y no se adivinan con `dlopen` a secas, por lo
/// mismo que [`super::stores::CANDIDATE_SOFTOKENS`]: cargar «la primera del
/// `LD_LIBRARY_PATH`» es dejar que el entorno decida.
pub const CANDIDATE_NSS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libnss3.so",
    "/usr/lib64/libnss3.so",
    "/usr/lib/libnss3.so",
];

/// Dónde puede estar `libsmime3.so`, que es donde viven **de verdad** los
/// `SEC_PKCS12Decoder*`.
pub const CANDIDATE_SMIME: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libsmime3.so",
    "/usr/lib64/libsmime3.so",
    "/usr/lib/libsmime3.so",
];

/// `SECSuccess`, que es lo único que devuelve bien una función de NSS.
const SEC_SUCCESS: c_int = 0;
/// `PR_TRUE`.
const PR_TRUE: c_int = 1;
/// `siBuffer`, el tipo de `SECItem` para una tira de bytes cualquiera.
const SI_BUFFER: c_uint = 0;

/// Los init args con los que se **crea** el almacén de un `.p12`.
///
/// `readWrite` porque aquí se escribe; el mismo directorio se abre luego en
/// solo lectura por [`super::stores::Store::nss`], que es como se lee todo lo
/// demás.
fn module_spec(directory: &Path) -> String {
    format!(
        "configDir='sql:{}' certPrefix='' keyPrefix='' \
         tokenDescription='rfirma' flags=readWrite",
        directory.display()
    )
}

/// El `SECItem` de NSS: tipo, bytes y longitud.
#[repr(C)]
struct SecItem {
    kind: c_uint,
    data: *mut c_uchar,
    len: c_uint,
}

/// El de la contraseña, con los bytes vivos al lado para que el puntero valga.
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

/// La contraseña tal y como la quiere el descodificador: UCS-2 *big endian* y
/// terminada en dos ceros, que es lo que un `BMPString` de PKCS#12 lleva
/// dentro.
///
/// La longitud **incluye** el terminador, igual que en `pk12util`.
fn bmp_string(password: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(password.len() * 2 + 2);
    for unit in password.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0]);
    bytes
}

/// Lo que NSS llama cuando le hace falta la contraseña de una ranura.
///
/// Devuelve siempre `NULL`, que para NSS es «no hay contraseña que dar». El
/// almacén se crea **sin contraseña de almacén** —ver [`import_pkcs12`]—, así
/// que este camino no se recorre; registrarlo igual es lo que garantiza que
/// ninguna ruta de NSS se quede esperando a un diálogo que aquí no existe.
extern "C" fn no_password(
    _slot: *mut c_void,
    _retry: c_int,
    _argument: *mut c_void,
) -> *mut c_char {
    std::ptr::null_mut()
}

/// Lo que NSS llama cuando el apodo de un certificado ya está cogido en el
/// almacén de destino.
///
/// Aquí el almacén de destino es de un solo fichero y acaba de crearse, así
/// que no hay con qué chocar. Se registra por lo mismo que [`no_password`]: un
/// `NULL` en su sitio sería una llamada a la dirección cero.
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

/// Las dos bibliotecas de NSS, cargadas una vez para todo el proceso.
///
/// No se descargan nunca a propósito: NSS deja estado global —la función de
/// contraseña, entre otras— y descargar el `.so` que lo sostiene mientras el
/// proceso sigue vivo es un fallo de los que no dejan rastro.
struct Nss {
    nss: Library,
    smime: Library,
}

static LIBRARIES: OnceLock<Result<Nss, String>> = OnceLock::new();

fn libraries() -> Result<&'static Nss, TokenError> {
    let loaded = LIBRARIES.get_or_init(|| {
        let nss = first_present(CANDIDATE_NSS, "libnss3.so")?;
        let smime = first_present(CANDIDATE_SMIME, "libsmime3.so")?;
        Ok(Nss { nss, smime })
    });
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

/// El puntero a una función de una biblioteca ya cargada.
///
/// Devuelve el fallo con el nombre del símbolo dentro: una NSS a la que le
/// falte uno de los quince es un diagnóstico, no un `None` cualquiera.
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

/// Un fallo de una llamada a NSS, con el paso dentro.
///
/// No lleva el `PR_GetError()` de NSPR: sacarlo obligaría a cargar una tercera
/// biblioteca, y el código que devuelve **no es de fiar cuando la llamada tuvo
/// éxito** —`PK11_InitPin` devuelve `SECSuccess` dejando un error viejo puesto,
/// medido en el sondeo—, así que lo que se dice es qué paso falló.
fn failed(step: &str) -> TokenError {
    TokenError::new(
        Situation::Pkcs12Unreadable,
        format!("NSS ha fallado en {step}"),
    )
}

/// **Mete el `.p12` en un almacén NSS recién creado en `directory`.**
///
/// `password` es la contraseña **del fichero**, la que teclea quien lo
/// instala; se usa para descifrarlo y no se guarda en ninguna parte (ID-196).
/// El almacén queda **sin contraseña de almacén**, que es lo que hace que sus
/// certificados se listen luego sin teclear nada (ID-195): la protección real
/// de un `.p12` instalado es la del directorio de datos de la aplicación, con
/// sus permisos, y no una segunda contraseña que nadie ha pedido.
///
/// # Tiene que llamarse dentro del turno del token
///
/// [`super::with_token_turn`] es lo único que garantiza que `libsoftokn3.so` no
/// está inicializado mientras NSS vive (ID-194). Fuera del turno esto no falla:
/// lista el almacén equivocado, que es peor.
pub fn import_pkcs12(directory: &Path, pkcs12: &[u8], password: &str) -> Result<(), TokenError> {
    let libraries = libraries()?;

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

    let nss_no_db_init: NoDbInit = symbol(&libraries.nss, b"NSS_NoDB_Init\0")?;
    let nss_shutdown: Shutdown = symbol(&libraries.nss, b"NSS_Shutdown\0")?;
    let open_user_db: OpenUserDb = symbol(&libraries.nss, b"SECMOD_OpenUserDB\0")?;
    let close_user_db: CloseUserDb = symbol(&libraries.nss, b"SECMOD_CloseUserDB\0")?;
    let need_user_init: NeedUserInit = symbol(&libraries.nss, b"PK11_NeedUserInit\0")?;
    let init_pin: InitPin = symbol(&libraries.nss, b"PK11_InitPin\0")?;
    let authenticate: Authenticate = symbol(&libraries.nss, b"PK11_Authenticate\0")?;
    let free_slot: FreeSlot = symbol(&libraries.nss, b"PK11_FreeSlot\0")?;
    let set_password_func: SetPasswordFunc = symbol(&libraries.nss, b"PK11_SetPasswordFunc\0")?;
    let decoder_start: DecoderStart = symbol(&libraries.smime, b"SEC_PKCS12DecoderStart\0")?;
    let decoder_update: DecoderUpdate = symbol(&libraries.smime, b"SEC_PKCS12DecoderUpdate\0")?;
    let decoder_verify: DecoderStep = symbol(&libraries.smime, b"SEC_PKCS12DecoderVerify\0")?;
    let decoder_validate: DecoderValidate =
        symbol(&libraries.smime, b"SEC_PKCS12DecoderValidateBags\0")?;
    let decoder_import: DecoderStep = symbol(&libraries.smime, b"SEC_PKCS12DecoderImportBags\0")?;
    let decoder_finish: DecoderFinish = symbol(&libraries.smime, b"SEC_PKCS12DecoderFinish\0")?;

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
             inicializado el softoken (ID-194, ¿RFIRMA_PKCS11_MODULE apuntando a \
             libsoftokn3.so?)",
        ));
    }

    // A partir de aquí NSS está vivo y **todos** los caminos de salida tienen
    // que apagarlo: dejarlo encendido es dejar el proceso listando el almacén
    // equivocado (ID-194).
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
                // El primero que falla cuando la contraseña no es la del
                // fichero: la comprobación de integridad va con ella.
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
mod tests {
    use super::{bmp_string, module_spec};
    use std::path::Path;

    /// **Grada A**: son cadenas, no llaman a NSS.
    #[test]
    fn the_password_travels_as_a_big_endian_bmp_string_with_its_terminator() {
        assert_eq!(
            bmp_string("1234"),
            vec![0, b'1', 0, b'2', 0, b'3', 0, b'4', 0, 0]
        );
    }

    /// Un carácter fuera de ASCII ocupa sus dos bytes en el mismo orden, que es
    /// lo que separa un `BMPString` de una cadena de bytes.
    #[test]
    fn a_password_outside_ascii_keeps_the_big_endian_order() {
        assert_eq!(bmp_string("ñ"), vec![0x00, 0xf1, 0, 0]);
    }

    #[test]
    fn an_empty_password_is_just_the_terminator() {
        assert_eq!(bmp_string(""), vec![0, 0]);
    }

    /// El almacén se crea en formato `sql:` y en lectura y escritura: es el
    /// único momento en el que rfirma escribe dentro de un almacén NSS.
    #[test]
    fn the_store_is_created_in_sql_format_and_writable() {
        let spec = module_spec(Path::new("/casa/datos/rfirma/certificates/abc"));

        assert!(spec.contains("configDir='sql:/casa/datos/rfirma/certificates/abc'"));
        assert!(spec.contains("flags=readWrite"));
    }
}
