//! **Cómo entra la CA local en un almacén NSS de la persona** (ID-227, ID-228).
//!
//! Por la API de NSS y no por `certutil`: el binario de `libnss3-tools` está en
//! esta máquina de desarrollo, pero **no en el runtime del flatpak**, y el
//! ADR-0004 manda que los tres canales ejecuten literalmente el mismo código.
//! `libnss3.so` sí está en los tres —`org.gnome.Platform` lo trae—, que es lo
//! mismo que ya explota [`crate::pkcs11::nss`] para meter un `.p12`.
//!
//! # Los mismos dos cuidados que el importador de `.p12`
//!
//! Están medidos en `docs/research/p12-en-almacen-nss.md` y valen igual aquí:
//!
//! - **`NSS_Init` sobre un `configdir` y el `C_Initialize` de `cryptoki` no
//!   pueden convivir** (ID-194). Por eso se abre el perfil con `NSS_NoDB_Init`
//!   más `SECMOD_OpenUserDB`, y **dentro** de [`crate::pkcs11::with_token_turn`]
//!   —que aquí se toma solo, sin que quien llama tenga que acordarse—.
//! - **Todos los caminos de salida apagan NSS.** Dejarlo encendido deja al
//!   proceso listando el almacén equivocado, sin ningún error.
//!
//! # Lo que hace, y lo que no
//!
//! Hace lo mismo que `certutil -A -t "C,,"`: `PK11_ImportCert` con
//! `includeTrust` en falso y después `CERT_ChangeCertTrust`, que es el orden
//! del propio `certutil`. Los bits son [`TRUSTED_SSL_CA`] y **solo** esos: la
//! CA local no vale para correo ni para firma de código, y `TCP,TCP,TCP` —lo
//! que pone AutoFirma— le regala una confianza que nadie le ha pedido.
//!
//! **No borra nada.** No hay ninguna llamada de retirada en este fichero, y esa
//! ausencia es el solape del ID-224: instalar la CA siguiente deja la vigente
//! donde estaba. Cuando se escriba la retirada de Preferencias irá **por huella
//! del certificado y nunca por apodo**, que es literalmente el fallo medido en
//! el #225.
//!
//! # El apodo se comparte a propósito
//!
//! Las dos CA locales que conviven durante el solape llevan el **mismo** apodo,
//! porque en NSS el apodo va con el **sujeto** y no con el certificado: dos
//! certificados del mismo sujeto con apodos distintos es lo que NSS rechaza, no
//! lo contrario.

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void, CString};
use std::path::Path;
use std::sync::OnceLock;

use libloading::Library;

use super::error::{Situation, TrustError};
use super::TrustStores;
use crate::pkcs11::nss::CANDIDATE_NSS;
use crate::pkcs11::stores::present_among;
use crate::pkcs11::with_token_turn;

/// `SECSuccess`, que es lo único que devuelve bien una función de NSS.
const SEC_SUCCESS: c_int = 0;
/// `PR_FALSE`.
const PR_FALSE: c_int = 0;
/// `PR_TRUE`.
const PR_TRUE: c_int = 1;
/// `siBuffer`, el tipo de `SECItem` para una tira de bytes cualquiera.
const SI_BUFFER: c_uint = 0;
/// `CK_INVALID_HANDLE`: el certificado entra **sin** clave privada emparejada.
const NO_KEY: c_ulong = 0;

/// `CERTDB_VALID_CA`.
const CERTDB_VALID_CA: u32 = 0x0008;
/// `CERTDB_TRUSTED_CA`.
const CERTDB_TRUSTED_CA: u32 = 0x0010;

/// Los bits que hacen a un certificado **una CA de confianza para TLS**, y
/// nada más.
///
/// Es lo que `certutil -L` enseña como `C,,`. Ni `T` (`CERTDB_TRUSTED_CLIENT_CA`,
/// que es confianza para certificados de cliente) ni las dos columnas de la
/// derecha: la CA local firma un `CN=localhost` y no avala correo de nadie.
pub const TRUSTED_SSL_CA: u32 = CERTDB_VALID_CA | CERTDB_TRUSTED_CA;

/// **Si esos bits son los de una CA de confianza para TLS.**
///
/// No es `== TRUSTED_SSL_CA`, y esa tentación cuesta una tarde: lo que se
/// escribe y lo que se lee **no son el mismo número**. La confianza no vive en
/// `cert9.db` como una máscara de bits, sino como un `CKA_TRUST_SERVER_AUTH`
/// del softoken, y al volver de ahí un `CKT_NSS_TRUSTED_DELEGATOR` se
/// reconstruye siempre con `CERTDB_NS_TRUSTED_CA` puesto encima. Se comprueban
/// los dos bits que importan y se ignora el resto.
pub fn is_trusted_ssl_ca(flags: u32) -> bool {
    flags & TRUSTED_SSL_CA == TRUSTED_SSL_CA
}

/// Los init args con los que se abre el perfil de la persona **para escribir**.
///
/// Sin `tokenDescription`: aquí no se crea nada, se abre lo que ya existe, y
/// renombrarle el token al perfil de un Firefox ajeno sería tocar lo que no es
/// nuestro.
fn read_write_spec(profile: &Path) -> String {
    format!(
        "configDir='sql:{}' certPrefix='' keyPrefix='' flags=readWrite",
        profile.display()
    )
}

/// El `SECItem` de NSS: tipo, bytes y longitud.
#[repr(C)]
struct SecItem {
    kind: c_uint,
    data: *mut c_uchar,
    len: c_uint,
}

/// El `CERTCertTrust` de NSS: tres juegos de bits, uno por uso.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CertTrust {
    ssl: c_uint,
    email: c_uint,
    object_signing: c_uint,
}

/// Lo que NSS llama cuando le hace falta la contraseña de una ranura.
///
/// Devuelve siempre `NULL`, que para NSS es «no hay contraseña que dar».
/// Escribir un certificado **sin clave privada** en `cert9.db` no pide sesión
/// iniciada, así que este camino no se recorre; registrarlo igual es lo que
/// garantiza que ningún perfil con contraseña maestra deje a rfirma esperando
/// a un diálogo que aquí no existe.
extern "C" fn no_password(
    _slot: *mut c_void,
    _retry: c_int,
    _argument: *mut c_void,
) -> *mut c_char {
    std::ptr::null_mut()
}

static LIBRARY: OnceLock<Result<Library, String>> = OnceLock::new();

/// `libnss3.so`, cargada una vez para todo el proceso y nunca descargada, por
/// lo mismo que en [`crate::pkcs11::nss`]: NSS deja estado global.
fn library() -> Result<&'static Library, TrustError> {
    let loaded = LIBRARY.get_or_init(|| {
        let path = present_among(CANDIDATE_NSS, |path| path.is_file())
            .into_iter()
            .next()
            .ok_or_else(|| "no esta libnss3.so en ninguna de las rutas conocidas".to_owned())?;
        // SAFETY: la ruta sale de la lista cerrada de candidatos de
        // `pkcs11::nss`, no del entorno, y la biblioteca es la NSS del sistema.
        unsafe { Library::new(&path) }.map_err(|error| format!("{}: {error}", path.display()))
    });
    loaded
        .as_ref()
        .map_err(|detail| TrustError::new(Situation::NssMissing, detail.clone()))
}

fn symbol<T: Copy>(library: &'static Library, name: &[u8]) -> Result<T, TrustError> {
    // SAFETY: cada tipo `T` de este módulo es la firma declarada en la cabecera
    // pública de NSS para ese símbolo, y la biblioteca vive hasta que muere el
    // proceso.
    unsafe { library.get::<T>(name) }
        .map(|symbol| *symbol)
        .map_err(|error| {
            TrustError::new(
                Situation::NssMissing,
                format!(
                    "NSS no exporta «{}»: {error}",
                    String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
                ),
            )
        })
}

fn failed(situation: Situation, step: &str) -> TrustError {
    TrustError::new(situation, format!("NSS ha fallado en {step}"))
}

type NoDbInit = extern "C" fn(*const c_char) -> c_int;
type Shutdown = extern "C" fn() -> c_int;
type OpenUserDb = extern "C" fn(*const c_char) -> *mut c_void;
type CloseUserDb = extern "C" fn(*mut c_void) -> c_int;
type FreeSlot = extern "C" fn(*mut c_void);
type SetPasswordFunc = extern "C" fn(extern "C" fn(*mut c_void, c_int, *mut c_void) -> *mut c_char);
type DefaultCertDb = extern "C" fn() -> *mut c_void;
type NewTempCertificate =
    extern "C" fn(*mut c_void, *mut SecItem, *const c_char, c_int, c_int) -> *mut c_void;
type FindCertByDerCert = extern "C" fn(*mut c_void, *mut SecItem) -> *mut c_void;
type ImportCert = extern "C" fn(*mut c_void, *mut c_void, c_ulong, *const c_char, c_int) -> c_int;
type ChangeCertTrust = extern "C" fn(*mut c_void, *mut c_void, *mut CertTrust) -> c_int;
type GetCertTrust = extern "C" fn(*const c_void, *mut CertTrust) -> c_int;
type DestroyCertificate = extern "C" fn(*mut c_void);

/// Los doce símbolos de NSS que hacen falta aquí, resueltos de una vez.
struct Api {
    no_db_init: NoDbInit,
    shutdown: Shutdown,
    open_user_db: OpenUserDb,
    close_user_db: CloseUserDb,
    free_slot: FreeSlot,
    set_password_func: SetPasswordFunc,
    default_cert_db: DefaultCertDb,
    new_temp_certificate: NewTempCertificate,
    find_cert_by_der_cert: FindCertByDerCert,
    import_cert: ImportCert,
    change_cert_trust: ChangeCertTrust,
    get_cert_trust: GetCertTrust,
    destroy_certificate: DestroyCertificate,
}

impl Api {
    fn resolve() -> Result<Self, TrustError> {
        let nss = library()?;
        Ok(Self {
            no_db_init: symbol(nss, b"NSS_NoDB_Init\0")?,
            shutdown: symbol(nss, b"NSS_Shutdown\0")?,
            open_user_db: symbol(nss, b"SECMOD_OpenUserDB\0")?,
            close_user_db: symbol(nss, b"SECMOD_CloseUserDB\0")?,
            free_slot: symbol(nss, b"PK11_FreeSlot\0")?,
            set_password_func: symbol(nss, b"PK11_SetPasswordFunc\0")?,
            default_cert_db: symbol(nss, b"CERT_GetDefaultCertDB\0")?,
            new_temp_certificate: symbol(nss, b"CERT_NewTempCertificate\0")?,
            find_cert_by_der_cert: symbol(nss, b"CERT_FindCertByDERCert\0")?,
            import_cert: symbol(nss, b"PK11_ImportCert\0")?,
            change_cert_trust: symbol(nss, b"CERT_ChangeCertTrust\0")?,
            get_cert_trust: symbol(nss, b"CERT_GetCertTrust\0")?,
            destroy_certificate: symbol(nss, b"CERT_DestroyCertificate\0")?,
        })
    }
}

/// Abre el perfil, hace el trabajo y **siempre** apaga NSS y cierra el perfil.
///
/// Se toma [`crate::pkcs11::with_token_turn`] aquí dentro: es el único sitio
/// donde se sabe que hace falta, y dejárselo a quien llama es dejarle un fallo
/// que no da error, solo un listado del almacén equivocado (ID-194).
fn within<T>(
    profile: &Path,
    work: impl FnOnce(&Api, *mut c_void) -> Result<T, TrustError>,
) -> Result<T, TrustError> {
    let api = Api::resolve()?;
    let spec = CString::new(read_write_spec(profile)).map_err(|_| {
        TrustError::new(
            Situation::StoreUnreachable,
            "la ruta del perfil lleva un cero dentro",
        )
    })?;

    with_token_turn(|| {
        (api.set_password_func)(no_password);

        if (api.no_db_init)(std::ptr::null()) != SEC_SUCCESS {
            return Err(TrustError::new(
                Situation::StoreUnreachable,
                "NSS no ha podido arrancar sin base de datos: algo del proceso tiene ya \
                 inicializado el softoken (ID-194)",
            ));
        }

        // A partir de aquí NSS está vivo y **todos** los caminos de salida
        // tienen que apagarlo.
        let outcome = (|| {
            let slot = (api.open_user_db)(spec.as_ptr());
            if slot.is_null() {
                return Err(TrustError::new(
                    Situation::StoreUnreachable,
                    format!(
                        "SECMOD_OpenUserDB no ha podido abrir «{}» en lectura y escritura \
                         (¿falta el permiso del manifiesto del flatpak, ID-228?)",
                        profile.display()
                    ),
                ));
            }

            let done = work(&api, slot);

            (api.close_user_db)(slot);
            (api.free_slot)(slot);
            done
        })();

        (api.shutdown)();
        outcome
    })
}

/// El `SECItem` que envuelve un DER prestado, sin copiarlo.
fn der_item(der: &mut [u8]) -> SecItem {
    SecItem {
        kind: SI_BUFFER,
        data: der.as_mut_ptr(),
        len: der.len() as c_uint,
    }
}

/// La implementación de verdad de [`TrustStores`]: NSS por FFI.
#[derive(Clone, Copy, Debug, Default)]
pub struct NssTrustStores;

impl TrustStores for NssTrustStores {
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        nickname: &str,
    ) -> Result<(), TrustError> {
        let nickname = CString::new(nickname).map_err(|_| {
            TrustError::new(
                Situation::TrustNotWritten,
                "el apodo de la CA local lleva un cero dentro",
            )
        })?;
        let mut der = certificate_der.to_vec();

        within(profile, |api, slot| {
            let handle = (api.default_cert_db)();
            let mut item = der_item(&mut der);

            // `copyDER` en verdadero: NSS se queda con su propia copia, así que
            // `der` puede morir al salir de aquí.
            let certificate =
                (api.new_temp_certificate)(handle, &mut item, std::ptr::null(), PR_FALSE, PR_TRUE);
            if certificate.is_null() {
                return Err(failed(
                    Situation::TrustNotWritten,
                    "CERT_NewTempCertificate (¿el certificado de la CA local no es DER?)",
                ));
            }

            let written = (|| {
                // El mismo orden que `certutil -A`: entra sin confianza y
                // después se le escribe.
                if (api.import_cert)(slot, certificate, NO_KEY, nickname.as_ptr(), PR_FALSE)
                    != SEC_SUCCESS
                {
                    return Err(failed(Situation::StoreUnreachable, "PK11_ImportCert"));
                }
                let mut trust = CertTrust {
                    ssl: TRUSTED_SSL_CA,
                    ..CertTrust::default()
                };
                if (api.change_cert_trust)(handle, certificate, &mut trust) != SEC_SUCCESS {
                    return Err(failed(Situation::TrustNotWritten, "CERT_ChangeCertTrust"));
                }
                Ok(())
            })();

            (api.destroy_certificate)(certificate);
            written
        })
    }

    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError> {
        let mut der = certificate_der.to_vec();

        within(profile, |api, _slot| {
            let handle = (api.default_cert_db)();
            let mut item = der_item(&mut der);

            let certificate = (api.find_cert_by_der_cert)(handle, &mut item);
            if certificate.is_null() {
                return Ok(None);
            }

            let mut trust = CertTrust::default();
            let read = (api.get_cert_trust)(certificate, &mut trust);
            (api.destroy_certificate)(certificate);

            if read != SEC_SUCCESS {
                return Err(failed(Situation::TrustNotWritten, "CERT_GetCertTrust"));
            }
            Ok(Some(trust.ssl))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: son cadenas, no llaman a NSS.
    ///
    /// El perfil se abre en formato `sql:` y en lectura y escritura, que es el
    /// permiso que el ID-228 le pide al manifiesto del flatpak.
    #[test]
    fn the_profile_is_opened_read_write_and_in_sql_format() {
        let spec = read_write_spec(Path::new("/home/quien/.mozilla/firefox/perfil"));

        assert!(spec.contains("configDir='sql:/home/quien/.mozilla/firefox/perfil'"));
        assert!(spec.contains("flags=readWrite"));
    }

    /// Abrir el perfil de otro **no le cambia el nombre al token**: lo que se
    /// abre ya existe y no es nuestro.
    #[test]
    fn opening_someone_elses_profile_does_not_rename_their_token() {
        assert!(!read_write_spec(Path::new("/tmp/perfil")).contains("tokenDescription"));
    }

    /// La CA local es de confianza **para TLS y para nada más**: es el `C,,` de
    /// `certutil`, no el `TCP,TCP,TCP` de AutoFirma.
    /// Lo que devuelve el softoken lleva `CERTDB_NS_TRUSTED_CA` de propina, y
    /// eso **sigue siendo** una CA de confianza para TLS.
    #[test]
    fn the_bits_that_come_back_from_the_softoken_still_read_as_trusted() {
        assert!(is_trusted_ssl_ca(0x38));
        assert!(!is_trusted_ssl_ca(0x08));
        assert!(!is_trusted_ssl_ca(0));
    }

    #[test]
    fn the_local_ca_is_trusted_for_tls_and_for_nothing_else() {
        let trust = CertTrust {
            ssl: TRUSTED_SSL_CA,
            ..CertTrust::default()
        };

        assert_eq!(trust.ssl, 0x18);
        assert!(is_trusted_ssl_ca(trust.ssl));
        assert_eq!(trust.email, 0);
        assert_eq!(trust.object_signing, 0);
    }
}
