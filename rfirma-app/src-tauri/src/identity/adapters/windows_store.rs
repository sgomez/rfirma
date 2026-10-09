//! `WindowsToken`, el Almacén de Windows detrás del puerto `Token`, por CNG (ADR-0035).

mod cng;
mod smart_card;

use std::path::Path;

use super::pkcs11::RealToken;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::readers::with_the_cards_present;
use crate::identity::domain::secret::{PinWarning, StoreSecret};
use crate::identity::domain::store::{is_a_card_key_provider, Store};
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

/// Los certificados con clave del almacén del usuario, sin las copias de una tarjeta que no está en el lector (ADR-0048).
fn signable_with_the_cards_present() -> Result<Vec<TokenCertificate>, TokenError> {
    let found = cng::signable_certificates()?;
    let copied_from_a_card = found.iter().any(|certificate| {
        certificate
            .reference()
            .key_provider()
            .is_some_and(is_a_card_key_provider)
    });
    if !copied_from_a_card {
        return Ok(found);
    }
    Ok(with_the_cards_present(
        found,
        &smart_card::certificates_on_the_cards(),
    ))
}

/// Los almacenes de esta máquina: solo el del usuario (ADR-0035).
pub fn from_environment() -> Vec<Store> {
    vec![user_store()]
}

/// El adaptador de [`Token`] en Windows: CNG para el almacén del usuario y [`RealToken`] para los demás.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsToken;

impl Token for WindowsToken {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if is_the_user_store(store) {
            return signable_with_the_cards_present();
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
            return signable_with_the_cards_present();
        }
        RealToken.list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        if is_the_user_store(&reference.store()) {
            return Ok(StoreSecret::NotNeeded);
        }
        RealToken.secret_of(reference)
    }

    fn pin_warning(&self, reference: &CertificateRef) -> Result<PinWarning, TokenError> {
        if is_the_user_store(&reference.store()) {
            return Ok(PinWarning::Quiet);
        }
        RealToken.pin_warning(reference)
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

    fn hold_one_login(&self, reference: &CertificateRef) {
        RealToken.hold_one_login(reference);
    }

    fn release_the_login(&self, reference: &CertificateRef) {
        RealToken.release_the_login(reference);
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
