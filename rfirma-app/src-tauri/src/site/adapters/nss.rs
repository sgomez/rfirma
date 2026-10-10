//! Registro de certificados y confianza en los almacenes NSS por su API y no por `certutil`, que no está en el flatpak, sobre el `NssHost` de `identity` (ADR-0005).

use std::ffi::{c_char, c_int, c_uchar, c_uint, c_ulong, c_void, CString};
use std::path::Path;

use libloading::Library;

use crate::identity::adapters::pkcs11::NssHost;
use crate::site::domain::trust_error::{Situation, TrustError};
use crate::site::ports::TrustStores;

use crate::site::domain::local_ca::has_the_local_ca_subject;
use crate::site::domain::trust::TRUSTED_SSL_CA;

const SEC_SUCCESS: c_int = 0;
const PR_FALSE: c_int = 0;
const PR_TRUE: c_int = 1;
const SI_BUFFER: c_uint = 0;
const NO_KEY: c_ulong = 0;

fn spec(profile: &Path, flags: &str) -> String {
    format!(
        "configDir='sql:{}' certPrefix='' keyPrefix='' flags={flags}",
        profile.display()
    )
}

fn read_write_spec(profile: &Path) -> String {
    spec(profile, "readWrite")
}

fn read_only_spec(profile: &Path) -> String {
    spec(profile, "readOnly")
}

#[repr(C)]
struct SecItem {
    kind: c_uint,
    data: *mut c_uchar,
    len: c_uint,
}

#[repr(C)]
struct PrCList {
    next: *mut PrCList,
    prev: *mut PrCList,
}

#[repr(C)]
struct CertList {
    links: PrCList,
    arena: *mut c_void,
}

#[repr(C)]
struct CertListNode {
    links: PrCList,
    certificate: *mut c_void,
    application_data: *mut c_void,
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
type DeletePermCertificate = extern "C" fn(*mut c_void) -> c_int;
type ListCertsInSlot = extern "C" fn(*mut c_void) -> *mut CertList;
type DestroyCertList = extern "C" fn(*mut CertList);
type GetCertificateDer = extern "C" fn(*mut c_void, *mut SecItem) -> c_int;

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
    delete_perm_certificate: DeletePermCertificate,
    list_certs_in_slot: ListCertsInSlot,
    destroy_cert_list: DestroyCertList,
    get_certificate_der: GetCertificateDer,
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
            delete_perm_certificate: symbol(nss, b"SEC_DeletePermCertificate\0")?,
            list_certs_in_slot: symbol(nss, b"PK11_ListCertsInSlot\0")?,
            destroy_cert_list: symbol(nss, b"CERT_DestroyCertList\0")?,
            get_certificate_der: symbol(nss, b"CERT_GetCertificateDer\0")?,
        })
    }
}

fn certificates_in(api: &Api, list: *mut CertList) -> Vec<Vec<u8>> {
    let mut found = Vec::new();
    if list.is_null() {
        return found;
    }

    // SAFETY: NSS devuelve una lista circular viva hasta que se destruye.
    unsafe {
        let head: *mut PrCList = &raw mut (*list).links;
        let mut link = (*head).next;
        while !link.is_null() && !std::ptr::eq(link, head) {
            let certificate = (*link.cast::<CertListNode>()).certificate;
            let mut der = SecItem {
                kind: SI_BUFFER,
                data: std::ptr::null_mut(),
                len: 0,
            };
            if !certificate.is_null()
                && (api.get_certificate_der)(certificate, &mut der) == SEC_SUCCESS
                && !der.data.is_null()
            {
                found.push(std::slice::from_raw_parts(der.data, der.len as usize).to_vec());
            }
            link = (*link).next;
        }
    }

    found
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
        self.opened(profile, read_write_spec(profile), work)
    }

    fn opened<T>(
        &self,
        profile: &Path,
        spec: String,
        work: impl FnOnce(&Api, *mut c_void) -> Result<T, TrustError>,
    ) -> Result<T, TrustError> {
        let nss = self.host.library().map_err(|unavailable| {
            TrustError::new(Situation::NssMissing, unavailable.detail().to_owned())
        })?;
        let api = Api::resolve(nss)?;
        let spec = CString::new(spec).map_err(|_| {
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
                            "SECMOD_OpenUserDB no ha podido abrir «{}»",
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

    fn withdraw(&self, profile: &Path, certificate_der: &[u8]) -> Result<(), TrustError> {
        let mut der = certificate_der.to_vec();

        self.within(profile, |api, _slot| {
            let handle = (api.default_cert_db)();
            let mut item = der_item(&mut der);

            let certificate = (api.find_cert_by_der_cert)(handle, &mut item);
            if certificate.is_null() {
                return Ok(());
            }

            let deleted = (api.delete_perm_certificate)(certificate);
            (api.destroy_certificate)(certificate);
            if deleted != SEC_SUCCESS {
                return Err(failed(
                    Situation::TrustNotWithdrawn,
                    "SEC_DeletePermCertificate",
                ));
            }
            Ok(())
        })
    }

    fn local_cas(&self, profile: &Path) -> Result<Vec<Vec<u8>>, TrustError> {
        self.opened(profile, read_only_spec(profile), |api, slot| {
            let list = (api.list_certs_in_slot)(slot);
            let found = certificates_in(api, list);
            if !list.is_null() {
                (api.destroy_cert_list)(list);
            }
            Ok(found
                .into_iter()
                .filter(|der| has_the_local_ca_subject(der))
                .collect())
        })
    }
}

#[cfg(test)]
mod tests;
