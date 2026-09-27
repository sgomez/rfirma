//! La CA local en el almacén raíz del usuario de Windows, con CryptoAPI (ADR-0035).

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr;

use windows_sys::Win32::Foundation::{GetLastError, ERROR_CANCELLED};
use windows_sys::Win32::Security::Cryptography::{
    CertAddEncodedCertificateToStore, CertCloseStore, CertCreateCertificateContext,
    CertDeleteCertificateFromStore, CertFindCertificateInStore, CertFreeCertificateContext,
    CertOpenStore, CERT_CONTEXT, CERT_FIND_EXISTING, CERT_STORE_ADD_USE_EXISTING,
    CERT_STORE_PROV_SYSTEM_W, CERT_SYSTEM_STORE_CURRENT_USER, HCERTSTORE, PKCS_7_ASN_ENCODING,
    X509_ASN_ENCODING,
};

use super::SYSTEM_ROOT_STORE;
use crate::site::domain::trust::TRUSTED_SSL_CA;
use crate::site::domain::trust_error::{Situation, TrustError};
use crate::site::ports::TrustStores;

const ENCODING: u32 = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;
const USER_STORE_PREFIX: &str = "cryptoapi:CurrentUser/";

/// Almacenes de sistema de `CurrentUser`, nombrados como `cryptoapi:CurrentUser/<almacén>`.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsUserStores;

/// El almacén donde se confía la CA local: Edge y Chrome lo leen, y Firefox con `enterprise_roots`.
pub fn trust_profiles() -> Vec<PathBuf> {
    vec![PathBuf::from(SYSTEM_ROOT_STORE)]
}

impl TrustStores for WindowsUserStores {
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        _nickname: &str,
    ) -> Result<(), TrustError> {
        let store = OpenStore::of(profile)?;
        let added = unsafe {
            CertAddEncodedCertificateToStore(
                store.0,
                ENCODING,
                certificate_der.as_ptr(),
                length_of(certificate_der)?,
                CERT_STORE_ADD_USE_EXISTING,
                ptr::null_mut(),
            )
        };
        if added == 0 {
            return Err(refused_or_failed(
                Situation::TrustNotWritten,
                "CertAddEncodedCertificateToStore",
            ));
        }
        Ok(())
    }

    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError> {
        let store = OpenStore::of(profile)?;
        Ok(store
            .find(certificate_der)?
            .map(|found| unsafe { CertFreeCertificateContext(found) })
            .map(|_| TRUSTED_SSL_CA))
    }

    fn withdraw(&self, profile: &Path, certificate_der: &[u8]) -> Result<(), TrustError> {
        let store = OpenStore::of(profile)?;
        let Some(found) = store.find(certificate_der)? else {
            return Ok(());
        };
        if unsafe { CertDeleteCertificateFromStore(found) } == 0 {
            return Err(refused_or_failed(
                Situation::TrustNotWithdrawn,
                "CertDeleteCertificateFromStore",
            ));
        }
        Ok(())
    }
}

/// El nombre del almacén de `CurrentUser` que designa el perfil, en UTF-16 terminado en cero.
pub fn user_store_name(profile: &Path) -> Option<Vec<u16>> {
    let name = profile.to_str()?.strip_prefix(USER_STORE_PREFIX)?;
    (!name.is_empty() && !name.contains('\0'))
        .then(|| name.encode_utf16().chain(std::iter::once(0)).collect())
}

fn length_of(der: &[u8]) -> Result<u32, TrustError> {
    u32::try_from(der.len()).map_err(|_| {
        TrustError::new(
            Situation::TrustNotWritten,
            "el certificado de la CA local no cabe en CryptoAPI",
        )
    })
}

fn refused_or_failed(situation: Situation, step: &str) -> TrustError {
    let code = unsafe { GetLastError() };
    if code == ERROR_CANCELLED || code == (0x8007_0000 | ERROR_CANCELLED) {
        return TrustError::new(
            situation,
            "se ha rechazado el aviso de seguridad de Windows",
        );
    }
    TrustError::new(
        situation,
        format!("CryptoAPI ha fallado en {step} (código {code:#010x})"),
    )
}

struct OpenStore(HCERTSTORE);

impl OpenStore {
    fn of(profile: &Path) -> Result<Self, TrustError> {
        let name = user_store_name(profile).ok_or_else(|| {
            TrustError::new(
                Situation::StoreUnreachable,
                format!("«{}» no es un almacén de Windows", profile.display()),
            )
        })?;
        let store = unsafe {
            CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                0,
                0,
                CERT_SYSTEM_STORE_CURRENT_USER,
                name.as_ptr().cast::<c_void>(),
            )
        };
        if store.is_null() {
            return Err(refused_or_failed(
                Situation::StoreUnreachable,
                "CertOpenStore",
            ));
        }
        Ok(Self(store))
    }

    fn find(&self, der: &[u8]) -> Result<Option<*const CERT_CONTEXT>, TrustError> {
        let wanted =
            unsafe { CertCreateCertificateContext(ENCODING, der.as_ptr(), length_of(der)?) };
        if wanted.is_null() {
            return Err(TrustError::new(
                Situation::StoreUnreachable,
                "el certificado de la CA local no es DER",
            ));
        }
        let found = unsafe {
            CertFindCertificateInStore(
                self.0,
                ENCODING,
                0,
                CERT_FIND_EXISTING,
                wanted.cast::<c_void>(),
                ptr::null(),
            )
        };
        unsafe { CertFreeCertificateContext(wanted) };
        Ok((!found.is_null()).then_some(found))
    }
}

impl Drop for OpenStore {
    fn drop(&mut self) {
        unsafe { CertCloseStore(self.0, 0) };
    }
}

#[cfg(test)]
mod tests;
