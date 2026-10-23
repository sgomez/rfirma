use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rdnie::Session;
use rdnie_emulator::{EmulatedDnie, Step};

use super::super::{dnie_store, CardOpener, DnieToken};
use super::OfThePolice;
use crate::identity::application::certificates::{
    certificates_with_their_chains, rows_of, ListedCertificates,
};
use crate::identity::application::readers::{chosen_from_the_last_listing, LastListing};
use crate::identity::application::tests::{NoMemory, NoToken, TestAuthority};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

/// Un DNIe que se lista bien la primera vez y, quitada la tarjeta, no está en las siguientes.
struct RemovedAfterListing {
    opened: Arc<AtomicUsize>,
}

impl CardOpener for RemovedAfterListing {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        let card = OfThePolice::new().on(EmulatedDnie::dnie_3());
        let card = match self.opened.fetch_add(1, Ordering::SeqCst) {
            0 => card,
            _ => card.removing_card_at(Step::Certificates),
        };
        let root = card.component_root();
        Session::open_trusting(card, root)
    }
}

/// El DNIe con, además del DNIe, un almacén que siempre responde.
struct WithAnotherStore {
    dnie: DnieToken<RemovedAfterListing>,
    another: Store,
}

impl Token for WithAnotherStore {
    fn list(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if *store != self.another {
            return self.dnie.list(store);
        }
        Ok(vec![TokenCertificate::new(
            CertificateRef::new(self.another.clone(), "otro", "OTRO", vec![0x02]),
            TestAuthority::root("Firmante del otro almacen").der(),
        )])
    }

    fn every_certificate(&self, store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        if *store != self.another {
            return self.dnie.every_certificate(store);
        }
        Ok(Vec::new())
    }

    fn list_authenticated(
        &self,
        store: &Store,
        pin: &ProtectedSecret,
    ) -> Result<Vec<TokenCertificate>, TokenError> {
        self.dnie.list_authenticated(store, pin)
    }

    fn secret_of(&self, reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Token::secret_of(&NoToken, reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Token::offers(&NoToken, reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        Token::accepts_the_secret(&NoToken, reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        Token::sign_with_secret(&NoToken, reference, secret, algorithm, data)
    }

    fn import_pkcs12(
        &self,
        directory: &std::path::Path,
        pkcs12: &[u8],
        password: &str,
        pin: &ProtectedSecret,
    ) -> Result<Store, TokenError> {
        NoToken.import_pkcs12(directory, pkcs12, password, pin)
    }

    fn remove_certificate(
        &self,
        directory: &std::path::Path,
        reference: &CertificateRef,
        pin: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        NoToken.remove_certificate(directory, reference, pin)
    }
}

#[test]
fn a_card_taken_out_between_the_listing_and_the_signature_is_an_absent_token_not_a_missing_certificate(
) {
    let another = Store::module("/usr/lib/another.so");
    let token = WithAnotherStore {
        dnie: DnieToken::new(RemovedAfterListing {
            opened: Arc::new(AtomicUsize::new(0)),
        }),
        another: another.clone(),
    };
    let stores = [dnie_store(), another];
    let installed = tempfile::tempdir().expect("directorio temporal");
    let listed = ListedCertificates::new();
    let found = certificates_with_their_chains(&token, &stores, &LastListing::default())
        .expect("la tarjeta esta al listar");
    let handle = rows_of(
        found,
        installed.path(),
        &listed,
        &ListedCertificates::new(),
        &NoMemory,
    )
    .into_iter()
    .find(|row| !row.label.contains("otro"))
    .expect("el DNIe tiene fila")
    .id;

    let failure =
        chosen_from_the_last_listing(&token, &stores, &LastListing::default(), &listed, &handle)
            .expect_err("la tarjeta ya no esta");

    assert_eq!(failure.situation(), Situation::TokenAbsent);
    assert_ne!(failure.situation(), Situation::CertificateNotFound);
}
