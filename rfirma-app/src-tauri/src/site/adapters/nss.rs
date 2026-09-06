//! Gestión de certificados y confianza en almacenes NSS mediante FFI (ADR-0005).

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void, CString};
use std::path::Path;

use libloading::Library;

use crate::identity::ports::NssHost;
use crate::site::domain::trust_error::{Situation, TrustError};
use crate::site::ports::TrustStores;

pub use crate::site::domain::trust::{is_trusted_ssl_ca, TRUSTED_SSL_CA};

const SEC_SUCCESS: c_int = 0;
const PR_FALSE: c_int = 0;
const PR_TRUE: c_int = 1;
const SI_BUFFER: c_uint = 0;
const NO_KEY: c_ulong = 0;

fn read_write_spec(profile: &Path) -> String {
    format!(
        "configDir='sql:{}' certPrefix='' keyPrefix='' flags=readWrite",
        profile.display()
    )
}

#[repr(C)]
struct SecItem {
    kind: c_uint,
    data: *mut c_uchar,
    len: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CertTrust {
    ssl: c_uint,
    email: c_uint,
    object_signing: c_uint,
}

extern "C" fn no_password(
    _slot: *mut c_void,
    _retry: c_int,
    _argument: *mut c_void,
) -> *mut c_char {
    std::ptr::null_mut()
}

fn symbol<T: Copy>(library: &'static Library, name: &[u8]) -> Result<T, TrustError> {
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
    fn resolve(nss: &'static Library) -> Result<Self, TrustError> {
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

fn der_item(der: &mut [u8]) -> SecItem {
    SecItem {
        kind: SI_BUFFER,
        data: der.as_mut_ptr(),
        len: der.len() as c_uint,
    }
}

/// Implementación de [`TrustStores`] mediante la API C de NSS por FFI.
#[derive(Clone, Copy, Debug)]
pub struct NssTrustStores<H> {
    host: H,
}

impl<H> NssTrustStores<H> {
    /// Construye el acceso a los almacenes NSS sobre el anfitrión indicado.
    pub const fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: NssHost> NssTrustStores<H> {
    fn within<T>(
        &self,
        profile: &Path,
        work: impl FnOnce(&Api, *mut c_void) -> Result<T, TrustError>,
    ) -> Result<T, TrustError> {
        let nss = self.host.library().map_err(|unavailable| {
            TrustError::new(Situation::NssMissing, unavailable.detail().to_owned())
        })?;
        let api = Api::resolve(nss)?;
        let spec = CString::new(read_write_spec(profile)).map_err(|_| {
            TrustError::new(
                Situation::StoreUnreachable,
                "la ruta del perfil lleva un cero dentro",
            )
        })?;

        self.host.with_token_turn(|| {
            (api.set_password_func)(no_password);

            if (api.no_db_init)(std::ptr::null()) != SEC_SUCCESS {
                return Err(TrustError::new(
                    Situation::StoreUnreachable,
                    "NSS no ha podido arrancar sin base de datos: el softoken ya está inicializado",
                ));
            }

            let outcome = (|| {
                let slot = (api.open_user_db)(spec.as_ptr());
                if slot.is_null() {
                    return Err(TrustError::new(
                        Situation::StoreUnreachable,
                        format!(
                            "SECMOD_OpenUserDB no ha podido abrir «{}» en lectura y escritura",
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
}

impl<H: NssHost> TrustStores for NssTrustStores<H> {
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

        self.within(profile, |api, slot| {
            let handle = (api.default_cert_db)();
            let mut item = der_item(&mut der);

            let certificate =
                (api.new_temp_certificate)(handle, &mut item, std::ptr::null(), PR_FALSE, PR_TRUE);
            if certificate.is_null() {
                return Err(failed(
                    Situation::TrustNotWritten,
                    "CERT_NewTempCertificate (¿el certificado de la CA local no es DER?)",
                ));
            }

            let written = (|| {
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

        self.within(profile, |api, _slot| {
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
mod tests;
