//! `WithTheDnie`, el token de una plataforma con el DNIe delante: el almacén del DNIe va a rdnie y cualquier otro, al token envuelto (ADR-0040, ADR-0047).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use super::dnie::{dnie_store, CardOpener, DnieToken, PcscCard};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::listed::Listed;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::{Store, StoreClass};
use crate::identity::ports::Token;

/// Los almacenes de la plataforma y, detrás, el del DNIe.
pub fn and_the_dnie_store(stores: Vec<Store>) -> Vec<Store> {
    stores
        .into_iter()
        .chain(std::iter::once(dnie_store()))
        .collect()
}

/// El DNIe detrás del puerto [`Token`], que además dice en qué lector lo ha reconocido rdnie.
trait Dnie: Token + Send + Sync {
    fn claimed_reader(&self) -> Option<String>;
    fn readers_with_a_card(&self) -> Option<Vec<String>>;
    fn readings(&self) -> u64;
    fn list_since(&self, store: &Store, arrived: u64) -> Result<Vec<TokenCertificate>, TokenError>;
}

impl<O: CardOpener> Dnie for DnieToken<O> {
    fn readings(&self) -> u64 {
        DnieToken::readings(self)
    }

    fn list_since(&self, store: &Store, arrived: u64) -> Result<Vec<TokenCertificate>, TokenError> {
        DnieToken::list_since(self, store, arrived)
    }

    fn claimed_reader(&self) -> Option<String> {
        DnieToken::claimed_reader(self)
    }

    fn readers_with_a_card(&self) -> Option<Vec<String>> {
        DnieToken::readers_with_a_card(self)
    }
}

/// El adaptador de [`Token`] que reparte entre rdnie, para el almacén del DNIe, y el token de la plataforma.
#[derive(Clone)]
pub struct WithTheDnie<T> {
    platform: T,
    dnie: Arc<dyn Dnie>,
    stood_in_for: Arc<Mutex<BTreeSet<PathBuf>>>,
}

impl<T: Token + Default> Default for WithTheDnie<T> {
    fn default() -> Self {
        Self::new(T::default(), DnieToken::new(PcscCard))
    }
}

impl<T: Token> WithTheDnie<T> {
    /// `platform`, con el DNIe que abre `dnie`.
    pub fn new<O: CardOpener + 'static>(platform: T, dnie: DnieToken<O>) -> Self {
        Self {
            platform,
            dnie: Arc::new(dnie),
            stood_in_for: Arc::default(),
        }
    }

    /// Los módulos que, en su último listado, se han saltado la ranura del DNIe.
    fn modules_stood_in_for(&self) -> Vec<PathBuf> {
        self.stood_in_for
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .cloned()
            .collect()
    }

    fn remember(&self, store: &Store, skipped_a_slot: bool) {
        let mut stood_in_for = self
            .stood_in_for
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if skipped_a_slot {
            stood_in_for.insert(store.path().to_path_buf());
        } else {
            stood_in_for.remove(store.path());
        }
    }

    fn holding(&self, store: &Store) -> &dyn Token {
        if store.is_the_dnie() {
            return self.dnie.as_ref();
        }
        &self.platform
    }

    /// El lector del DNIe, si el almacén es de tarjeta y rdnie ha reconocido uno: su ranura PKCS#11 no se abre.
    fn skipped_for(&self, store: &Store) -> Vec<String> {
        if store.class() != StoreClass::Card {
            return Vec::new();
        }
        self.dnie.claimed_reader().into_iter().collect()
    }

    /// OpenSC arranca su driver en cada tarjeta al enumerar ranuras: sin más tarjeta que el DNIe, ni se abre.
    fn sees_only_the_dnie(&self, store: &Store, skipped: &[String]) -> bool {
        !skipped.is_empty()
            && is_opensc(store)
            && self
                .dnie
                .readers_with_a_card()
                .is_some_and(|readers| readers.iter().all(|reader| skipped.contains(reader)))
    }
}

fn is_opensc(store: &Store) -> bool {
    store
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("opensc-pkcs11"))
}

impl<T: Token> WithTheDnie<T> {
    /// Lo que enseña `store` a un listado que empezó tras `arrived` lecturas del DNIe.
    fn list_arrived(
        &self,
        store: &Store,
        arrived: u64,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        if store.is_the_dnie() {
            let modules = self.modules_stood_in_for();
            return Ok(self
                .dnie
                .list_since(store, arrived)?
                .into_iter()
                .map(|certificate| certificate.standing_in_for(modules.clone()))
                .collect());
        }
        let skipped = self.skipped_for(store);
        if self.sees_only_the_dnie(store, &skipped) {
            self.remember(store, true);
            return Ok(Vec::new());
        }
        let listed = self.platform.list_skipping(store, &skipped)?;
        self.remember(store, listed.skipped_a_slot);
        Ok(listed.certificates)
    }
}

impl<T: Token> Token for WithTheDnie<T> {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        self.list_arrived(store, self.dnie.readings())
    }

    /// Un listado que empieza mientras se lee el DNIe se queda con esa lectura al llegar a su almacén.
    fn list_each(&self, stores: &[Store]) -> Listed {
        let arrived = self.dnie.readings();
        let mut listed = Listed::default();
        for store in stores {
            listed.heard(store, self.list_arrived(store, arrived));
        }
        listed
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if store.is_the_dnie() {
            return self.dnie.every_certificate(store);
        }
        let skipped = self.skipped_for(store);
        if self.sees_only_the_dnie(store, &skipped) {
            return Ok(Vec::new());
        }
        self.platform.every_certificate_skipping(store, &skipped)
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
