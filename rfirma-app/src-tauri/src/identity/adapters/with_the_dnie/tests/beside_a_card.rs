use fake_pkcs11::FakeCard;
use rdnie::Session;
use rdnie_emulator::EmulatedDnie;

use super::super::WithTheDnie;
use crate::identity::adapters::dnie::{CardOpener, DnieToken};
use crate::identity::adapters::pkcs11::RealToken;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;

const DNIE_READER: &str = "Lector de la mesa con el DNIe 00 00";
const LONG_DNIE_READER: &str =
    "Lector de nombre muy largo, el del DNIe, que no cabe en la ranura PKCS#11 00 00";
const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";

/// Un DNIe emulado que rdnie ha reconocido en el lector `reader`.
struct InTheReader {
    reader: &'static str,
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
}

fn with_the_dnie_in(reader: &'static str) -> WithTheDnie<RealToken> {
    WithTheDnie::new(RealToken, DnieToken::new(InTheReader { reader }))
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
