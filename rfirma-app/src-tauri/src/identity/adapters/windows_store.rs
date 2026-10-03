//! `WindowsToken`, los almacenes de Windows detrás del puerto `Token`: el del usuario por CNG y, para lo demás, los módulos PKCS#11 y dónde se buscan (ADR-0035).

mod cng;

use std::path::{Path, PathBuf};

use super::pkcs11::stores::present_among;
use super::pkcs11::RealToken;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

/// La ruta con la que se nombra `CurrentUser\MY`: ningún módulo PKCS#11 puede llamarse así.
pub const USER_STORE: &str = "cng:CurrentUser/MY";

/// El almacén de certificados personales del usuario.
pub fn user_store() -> Store {
    Store::module(USER_STORE)
}

fn is_the_user_store(store: &Store) -> bool {
    store.path() == Path::new(USER_STORE)
}

/// Los módulos PKCS#11 conocidos de OpenSC y del DNIe bajo `Program Files` y `System32`.
pub fn candidate_modules(program_files: &Path, system32: &Path) -> Vec<PathBuf> {
    vec![
        program_files.join("OpenSC Project/OpenSC/pkcs11/opensc-pkcs11.dll"),
        system32.join("opensc-pkcs11.dll"),
        system32.join("DNIe_P11_priv.dll"),
        system32.join("UsrPkcs11.dll"),
    ]
}

/// Los almacenes de esta máquina: primero el del usuario, que gana a las copias de PKCS#11 (ADR-0035).
pub fn from_environment() -> Vec<Store> {
    if let Some(module) = std::env::var_os(crate::PKCS11_MODULE_VARIABLE) {
        return vec![Store::module(module)];
    }
    let program_files = folder_from("ProgramFiles", r"C:\Program Files");
    let system32 = folder_from("SystemRoot", r"C:\Windows").join("System32");
    std::iter::once(user_store())
        .chain(
            present_among(candidate_modules(&program_files, &system32), Path::is_file)
                .into_iter()
                .map(Store::module),
        )
        .collect()
}

fn folder_from(variable: &str, otherwise: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(otherwise))
}

/// El adaptador de [`Token`] en Windows: CNG para el almacén del usuario y [`RealToken`] para los módulos.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsToken;

impl Token for WindowsToken {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_the_user_store(store) {
            return cng::signable_certificates();
        }
        RealToken.list(store)
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_the_user_store(store) {
            return cng::every_certificate();
        }
        RealToken.every_certificate(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_the_user_store(store) {
            return cng::signable_certificates();
        }
        RealToken.list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        if is_the_user_store(&reference.store()) {
            return Ok(StoreSecret::NotNeeded);
        }
        RealToken.secret_of(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        if is_the_user_store(&reference.store()) {
            return cng::offers(reference, algorithm);
        }
        RealToken.offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        if is_the_user_store(&reference.store()) {
            return Ok(());
        }
        RealToken.accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        if is_the_user_store(&reference.store()) {
            return cng::sign(reference, algorithm, data);
        }
        RealToken.sign_with_secret(reference, secret, algorithm, data)
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
        pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        RealToken.import_pkcs12(directory, pkcs12, password, pin)
    }

    fn remove_certificate(
        &self,
        directory: &Path,
        reference: &CertificateRef,
        pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        RealToken.remove_certificate(directory, reference, pin)
    }
}

#[cfg(test)]
mod tests;
