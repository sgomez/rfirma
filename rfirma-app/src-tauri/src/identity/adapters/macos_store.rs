//! `MacosToken`, el Almacén de macOS detrás del puerto `Token`, por Security.framework; los módulos PKCS#11 siguen en `RealToken` (ADR-0040).

mod installs;
mod keychain;

use std::path::Path;
use std::sync::Arc;

use super::dnie::{dnie_store, DnieToken, PcscCard};
use super::pkcs11::RealToken;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::{Store, StoreClass, MACOS_STORE_PREFIX};
use crate::identity::ports::Token;

const SEARCH_LIST: &str = "search-list";

/// El llavero de la persona usuaria: su lista de búsqueda entera.
pub fn user_store() -> Store {
    Store::module(format!("{MACOS_STORE_PREFIX}{SEARCH_LIST}"))
}

/// Un llavero suelto, por su fichero.
pub fn keychain_store(path: &Path) -> Store {
    Store::module(format!("{MACOS_STORE_PREFIX}{}", path.display()))
}

fn is_a_keychain(store: &Store) -> bool {
    store.class() == StoreClass::Macos
}

/// Los almacenes de esta máquina: el llavero y los PKCS#11 descubiertos, o solo el módulo nombrado, y el DNIe.
pub fn from_environment() -> Vec<Store> {
    stores_besides(
        super::pkcs11::stores::from_environment(),
        std::env::var_os(crate::PKCS11_MODULE_VARIABLE).is_some(),
    )
}

fn stores_besides(pkcs11: Vec<Store>, module_named: bool) -> Vec<Store> {
    let keychain = (!module_named).then(user_store);
    keychain
        .into_iter()
        .chain(pkcs11)
        .chain(std::iter::once(dnie_store()))
        .collect()
}

/// El adaptador de [`Token`] en macOS: el llavero para el Almacén de macOS, rdnie para el almacén del DNIe y [`RealToken`] para los demás.
#[derive(Clone)]
pub struct MacosToken {
    dnie: Arc<dyn Token + Send + Sync>,
}

impl Default for MacosToken {
    fn default() -> Self {
        Self::with_the_dnie(DnieToken::new(PcscCard))
    }
}

impl MacosToken {
    /// El de siempre, con el DNIe que abre otro.
    pub fn with_the_dnie(dnie: impl Token + Send + Sync + 'static) -> Self {
        Self {
            dnie: Arc::new(dnie),
        }
    }

    fn beyond_the_keychain(&self, store: &Store) -> &dyn Token {
        if store.is_the_dnie() {
            return self.dnie.as_ref();
        }
        &RealToken
    }
}

impl Token for MacosToken {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_a_keychain(store) {
            return keychain::signable_certificates(store);
        }
        self.beyond_the_keychain(store).list(store)
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_a_keychain(store) {
            return keychain::every_certificate(store);
        }
        self.beyond_the_keychain(store).every_certificate(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_a_keychain(store) {
            return keychain::signable_certificates(store);
        }
        self.beyond_the_keychain(store)
            .list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        if is_a_keychain(&reference.store()) {
            return Ok(StoreSecret::NotNeeded);
        }
        self.beyond_the_keychain(&reference.store())
            .secret_of(reference)
    }

    fn pin_warning(&self, reference: &CertificateRef) -> Result<PinWarning, TokenError> {
        if is_a_keychain(&reference.store()) {
            return Ok(PinWarning::Quiet);
        }
        self.beyond_the_keychain(&reference.store())
            .pin_warning(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        if is_a_keychain(&reference.store()) {
            return keychain::offers(reference, algorithm);
        }
        self.beyond_the_keychain(&reference.store())
            .offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        if is_a_keychain(&reference.store()) {
            return Ok(());
        }
        self.beyond_the_keychain(&reference.store())
            .accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        if is_a_keychain(&reference.store()) {
            return keychain::sign(reference, algorithm, data);
        }
        self.beyond_the_keychain(&reference.store())
            .sign_with_secret(reference, secret, algorithm, data)
    }

    fn hold_one_login(&self, reference: &CertificateRef) {
        self.beyond_the_keychain(&reference.store())
            .hold_one_login(reference);
    }

    fn release_the_login(&self, reference: &CertificateRef) {
        self.beyond_the_keychain(&reference.store())
            .release_the_login(reference);
    }

    fn installs_in_the_system(&self) -> bool {
        true
    }

    fn installed_in_the_system(&self, directory: &Path, reference: &CertificateRef) -> bool {
        is_a_keychain(&reference.store())
            && reference
                .cka_id()
                .is_some_and(|thumbprint| installs::holds(directory, thumbprint))
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
        _pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        let store = user_store();
        let thumbprints = keychain::import_pkcs12(&store, pkcs12, password)?;
        installs::record(directory, &thumbprints)?;
        Ok(store)
    }

    fn remove_certificate(
        &self,
        directory: &Path,
        reference: &CertificateRef,
        _pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        keychain::remove(reference)?;
        installs::forget(directory, reference.cka_id().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests;
