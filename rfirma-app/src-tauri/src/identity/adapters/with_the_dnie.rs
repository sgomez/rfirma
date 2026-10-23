//! `WithTheDnie`, el token de una plataforma con el DNIe delante: el almacén del DNIe va a rdnie y cualquier otro, al token envuelto (ADR-0040, ADR-0047).

use std::path::Path;
use std::sync::Arc;

use super::dnie::{dnie_store, DnieToken, PcscCard};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

/// Los almacenes de la plataforma y, detrás, el del DNIe.
pub fn and_the_dnie_store(stores: Vec<Store>) -> Vec<Store> {
    stores
        .into_iter()
        .chain(std::iter::once(dnie_store()))
        .collect()
}

/// El adaptador de [`Token`] que reparte entre rdnie, para el almacén del DNIe, y el token de la plataforma.
#[derive(Clone)]
pub struct WithTheDnie<T> {
    platform: T,
    dnie: Arc<dyn Token + Send + Sync>,
}

impl<T: Token + Default> Default for WithTheDnie<T> {
    fn default() -> Self {
        Self::new(T::default(), DnieToken::new(PcscCard))
    }
}

impl<T: Token> WithTheDnie<T> {
    /// `platform`, con el DNIe que abre `dnie`.
    pub fn new(platform: T, dnie: impl Token + Send + Sync + 'static) -> Self {
        Self {
            platform,
            dnie: Arc::new(dnie),
        }
    }

    fn holding(&self, store: &Store) -> &dyn Token {
        if store.is_the_dnie() {
            return self.dnie.as_ref();
        }
        &self.platform
    }
}

impl<T: Token> Token for WithTheDnie<T> {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.holding(store).list(store)
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.holding(store).every_certificate(store)
    }

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        self.holding(store).list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        self.holding(&reference.store()).secret_of(reference)
    }

    fn pin_warning(&self, reference: &CertificateRef) -> Result<PinWarning, TokenError> {
        self.holding(&reference.store()).pin_warning(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        self.holding(&reference.store())
            .offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        self.holding(&reference.store())
            .accepts_the_secret(reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        self.holding(&reference.store())
            .sign_with_secret(reference, secret, algorithm, data)
    }

    fn hold_one_login(&self, reference: &CertificateRef) {
        self.holding(&reference.store()).hold_one_login(reference);
    }

    fn release_the_login(&self, reference: &CertificateRef) {
        self.holding(&reference.store())
            .release_the_login(reference);
    }

    fn installs_in_the_system(&self) -> bool {
        self.platform.installs_in_the_system()
    }

    fn installed_in_the_system(&self, directory: &Path, reference: &CertificateRef) -> bool {
        self.platform.installed_in_the_system(directory, reference)
    }

    fn import_pkcs12(
        &self,
        directory: &Path,
        pkcs12: &[u8],
        password: &str,
        pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        self.platform
            .import_pkcs12(directory, pkcs12, password, pin)
    }

    fn remove_certificate(
        &self,
        directory: &Path,
        reference: &CertificateRef,
        pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        self.platform.remove_certificate(directory, reference, pin)
    }
}

#[cfg(test)]
mod tests;
