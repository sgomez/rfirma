use std::path::Path;

use fake_pkcs11::FakeCard;
use rdnie::Session;
use rdnie_emulator::EmulatedDnie;

use super::super::WithTheDnie;
use crate::identity::adapters::dnie::{dnie_store, CardOpener, DnieToken};
use crate::identity::adapters::pkcs11::RealToken;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

const DNIE_READER: &str = "Lector de la mesa con el DNIe 00 00";
const LONG_DNIE_READER: &str =
    "Lector de nombre muy largo, el del DNIe, que no cabe en la ranura PKCS#11 00 00";
const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";
const SOFTHSM: &str = "/usr/lib/softhsm/libsofthsm2.so";

/// Un DNIe emulado que rdnie ha reconocido en el lector `reader`, con las tarjetas que ve PC/SC si se sabe.
struct InTheReader {
    reader: &'static str,
    with_a_card: Option<&'static [&'static str]>,
}

impl CardOpener for InTheReader {
    type Card = EmulatedDnie;

    fn open(&self) -> Result<Session<EmulatedDnie>, rdnie::Error> {
        let card = EmulatedDnie::dnie_4();
        let root = card.component_root();
        Session::open_trusting(card, root)
    }

    fn claimed_reader(&self) -> Option<String> {
        Some(self.reader.to_owned())
    }

    fn readers_with_a_card(&self) -> Option<Vec<String>> {
        self.with_a_card
            .map(|readers| readers.iter().map(|&reader| reader.to_owned()).collect())
    }
}

fn with_the_dnie_in(reader: &'static str) -> WithTheDnie<RealToken> {
    WithTheDnie::new(
        RealToken,
        DnieToken::new(InTheReader {
            reader,
            with_a_card: None,
        }),
    )
}

fn with_the_dnie_and_the_cards_in(
    reader: &'static str,
    with_a_card: &'static [&'static str],
) -> WithTheDnie<RealToken> {
    WithTheDnie::new(
        RealToken,
        DnieToken::new(InTheReader {
            reader,
            with_a_card: Some(with_a_card),
        }),
    )
}

/// El módulo de la tarjeta falsa con el nombre del de OpenSC.
fn as_opensc(card: &FakeCard) -> Store {
    let opensc = card.module().with_file_name("opensc-pkcs11.so");
    std::os::unix::fs::symlink(card.module(), &opensc).expect("el enlace deberia crearse");
    Store::module(opensc)
}

fn a_card_in(reader: &str) -> FakeCard {
    FakeCard::new()
        .and_then(|card| card.in_reader(reader))
        .expect("la tarjeta falsa deberia montarse")
}

fn labels_of(found: &[TokenCertificate]) -> Vec<&str> {
    found
        .iter()
        .map(|certificate| certificate.reference().label())
        .collect()
}

#[test]
fn the_slot_of_the_reader_rdnie_claimed_is_never_opened() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_in(DNIE_READER);
    let store = Store::module(card.module());

    let listed = token.list(&store).expect("el almacen deberia responder");
    let every = token
        .every_certificate(&store)
        .expect("el almacen deberia responder");

    assert_eq!(labels_of(&listed), Vec::<&str>::new());
    assert_eq!(labels_of(&every), Vec::<&str>::new());
    assert_eq!(card.calls_to("C_OpenSession"), Vec::<String>::new());
}

#[test]
fn a_card_in_another_reader_is_still_listed() {
    let card = a_card_in("Otro lector 01 00");
    let token = with_the_dnie_in(DNIE_READER);
    let store = Store::module(card.module());

    let listed = token.list(&store).expect("la tarjeta deberia listarse");
    let every = token
        .every_certificate(&store)
        .expect("la tarjeta deberia listarse");

    assert!(labels_of(&listed).contains(&SIGNING_CERTIFICATE));
    assert!(labels_of(&every).contains(&SIGNING_CERTIFICATE));
}

#[test]
fn a_reader_name_cut_to_the_slot_description_is_skipped_as_well() {
    let card = a_card_in(LONG_DNIE_READER);
    let token = with_the_dnie_in(LONG_DNIE_READER);
    let store = Store::module(card.module());

    let listed = token.list(&store).expect("el almacen deberia responder");
    let every = token
        .every_certificate(&store)
        .expect("el almacen deberia responder");

    assert_eq!(labels_of(&listed), Vec::<&str>::new());
    assert_eq!(labels_of(&every), Vec::<&str>::new());
    assert_eq!(card.calls_to("C_OpenSession"), Vec::<String>::new());
}

#[test]
fn the_dnie_stands_in_for_the_module_that_skipped_its_slot_and_for_no_other() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_in(DNIE_READER);
    let stores = [Store::module(card.module()), dnie_store()];

    let found = token
        .list_across(&stores)
        .expect("el DNIe deberia listarse");

    assert!(labels_of(&found).contains(&SIGNING_CERTIFICATE));
    assert!(found
        .iter()
        .all(|certificate| certificate.is_served_by(&card.module())));
    assert!(!found
        .iter()
        .any(|certificate| certificate.is_served_by(Path::new(SOFTHSM))));
}

#[test]
fn the_dnie_stands_in_for_no_module_when_none_skipped_its_slot() {
    let card = a_card_in("Otro lector 01 00");
    let token = with_the_dnie_in(DNIE_READER);

    token
        .list(&Store::module(card.module()))
        .expect("la tarjeta deberia listarse");
    let from_the_dnie = token.list(&dnie_store()).expect("el DNIe deberia listarse");

    assert!(!from_the_dnie.is_empty());
    assert!(!from_the_dnie
        .iter()
        .any(|certificate| certificate.is_served_by(&card.module())));
}

#[test]
fn opensc_is_not_opened_when_the_only_card_is_the_dnie() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_and_the_cards_in(DNIE_READER, &[DNIE_READER]);
    let opensc = as_opensc(&card);

    let listed = token.list(&opensc).expect("el almacen deberia responder");
    let every = token
        .every_certificate(&opensc)
        .expect("el almacen deberia responder");
    let from_the_dnie = token.list(&dnie_store()).expect("el DNIe deberia listarse");

    assert_eq!(labels_of(&listed), Vec::<&str>::new());
    assert_eq!(labels_of(&every), Vec::<&str>::new());
    assert_eq!(card.calls_to("C_Initialize"), Vec::<String>::new());
    assert!(from_the_dnie
        .iter()
        .all(|certificate| certificate.is_served_by(opensc.path())));
}

#[test]
fn opensc_is_opened_when_another_reader_has_a_card() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_and_the_cards_in(DNIE_READER, &[DNIE_READER, "Otro lector 01 00"]);

    token
        .list(&as_opensc(&card))
        .expect("el almacen deberia responder");

    assert_eq!(card.calls_to("C_Initialize").len(), 1, "{:?}", card.calls());
}

#[test]
fn a_module_other_than_opensc_is_opened_beside_the_dnie() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_and_the_cards_in(DNIE_READER, &[DNIE_READER]);

    token
        .list(&Store::module(card.module()))
        .expect("el almacen deberia responder");

    assert_eq!(card.calls_to("C_Initialize").len(), 1, "{:?}", card.calls());
}

#[test]
fn opensc_is_opened_when_the_cards_in_the_readers_are_unknown() {
    let card = a_card_in(DNIE_READER);
    let token = with_the_dnie_in(DNIE_READER);

    token
        .list(&as_opensc(&card))
        .expect("el almacen deberia responder");

    assert_eq!(card.calls_to("C_Initialize").len(), 1, "{:?}", card.calls());
}
