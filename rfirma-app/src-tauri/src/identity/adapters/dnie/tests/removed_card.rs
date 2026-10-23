use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rdnie::Session;
use rdnie_emulator::{EmulatedDnie, Step};

use super::super::{dnie_store, CardOpener, DnieToken};
use super::OfThePolice;
use crate::identity::adapters::with_the_dnie::WithTheDnie;
use crate::identity::application::certificates::{
    certificates_with_their_chains, rows_of, ListedCertificates,
};
use crate::identity::application::readers::{chosen_from_the_last_listing, LastListing};
use crate::identity::application::tests::{NoMemory, NoToken};
use crate::identity::domain::error::Situation;
use crate::identity::domain::store::Store;

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

#[test]
fn a_card_taken_out_between_the_listing_and_the_signature_is_an_absent_token_not_a_missing_certificate(
) {
    let another = Store::module("/usr/lib/another.so");
    let token = WithTheDnie::new(
        NoToken,
        DnieToken::new(RemovedAfterListing {
            opened: Arc::new(AtomicUsize::new(0)),
        }),
    );
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
    .next()
    .expect("el DNIe tiene fila")
    .id;

    let failure =
        chosen_from_the_last_listing(&token, &stores, &LastListing::default(), &listed, &handle)
            .expect_err("la tarjeta ya no esta");

    assert_eq!(failure.situation(), Situation::TokenAbsent);
    assert_ne!(failure.situation(), Situation::CertificateNotFound);
}
