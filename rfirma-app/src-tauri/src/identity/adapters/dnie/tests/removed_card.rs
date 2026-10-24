use rdnie_emulator::EmulatedDnie;

use super::in_the_reader::InTheReader;
use super::OfThePolice;
use crate::identity::adapters::dnie::dnie_store;
use crate::identity::adapters::with_the_dnie::WithTheDnie;
use crate::identity::application::certificates::{
    certificates_with_their_chains, rows_of, ListedCertificates,
};
use crate::identity::application::readers::{chosen_from_the_last_listing, LastListing};
use crate::identity::application::tests::{NoMemory, NoToken};
use crate::identity::domain::error::Situation;
use crate::identity::domain::store::Store;

#[test]
fn a_card_taken_out_between_the_listing_and_the_signature_is_an_absent_token_not_a_missing_certificate(
) {
    let another = Store::module("/usr/lib/another.so");
    let reader = InTheReader::new(OfThePolice::new().on(EmulatedDnie::dnie_3()));
    let token = WithTheDnie::new(NoToken, reader.token());
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
    reader.take_the_card_out();

    let failure =
        chosen_from_the_last_listing(&token, &stores, &LastListing::default(), &listed, &handle)
            .expect_err("la tarjeta ya no esta");

    assert_eq!(failure.situation(), Situation::TokenAbsent);
    assert_ne!(failure.situation(), Situation::CertificateNotFound);
}
