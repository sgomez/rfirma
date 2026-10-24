use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use rdnie::{Session, Transport, SIGNATURE_LABEL};
use rdnie_emulator::{EmulatedDnie, Step};

use super::super::{dnie_store, CardOpener, DnieToken};
use super::in_the_reader::{the_pin, the_signing_certificate, InTheReader};
use super::OfThePolice;
use crate::identity::application::tests::a_dnie_der;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::ports::Token;

const SIGNING_FILE: [u8; 2] = [0x70, 2];

fn signing_file_reads(reader: &InTheReader) -> usize {
    reader
        .card()
        .selected()
        .iter()
        .filter(|path| path.last() == Some(&SIGNING_FILE))
        .count()
}

fn signed_by(token: &dyn Token) -> Result<Vec<u8>, TokenError> {
    token.sign_with_secret(
        &the_signing_certificate(),
        &the_pin(),
        SignatureAlgorithm::Sha256Rsa,
        b"datos",
    )
}

#[test]
fn two_operations_in_a_row_open_the_card_once_and_read_no_certificate_twice() {
    let reader = InTheReader::new(OfThePolice::new().on(EmulatedDnie::dnie_4()));
    let dnie = reader.token();

    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");
    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");
    dnie.every_certificate(&dnie_store())
        .expect("el DNIe deberia listarse");

    assert_eq!(reader.openings(), 1);
    assert_eq!(signing_file_reads(&reader), 1);
}

#[test]
fn a_listing_struck_by_a_reset_of_another_program_is_listed_again_on_the_same_session() {
    let reader = InTheReader::new(
        OfThePolice::new()
            .on(EmulatedDnie::dnie_4())
            .resetting_card_at(Step::Certificates),
    );

    let found = reader
        .token()
        .list(&dnie_store())
        .expect("el listado deberia repetirse");

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reference().label(), SIGNATURE_LABEL);
    assert_eq!(reader.openings(), 1);
}

#[test]
fn a_signature_struck_by_a_reset_fails_without_a_second_verify() {
    let reader = InTheReader::new(
        OfThePolice::new()
            .on(EmulatedDnie::dnie_4())
            .resetting_card_at(Step::Signing),
    );

    let failure = signed_by(&reader.token()).expect_err("la tarjeta se ha reseteado");

    assert_eq!(failure.situation(), Situation::TokenAbsent);
    assert_eq!(reader.pins_sent(), 1);
}

/// Un lector donde se puede sacar la tarjeta y meter otra: cada sesión habla con la que había al abrirse.
#[derive(Clone)]
struct Swappable {
    inserted: Arc<Mutex<Arc<Mutex<EmulatedDnie>>>>,
    openings: Arc<AtomicUsize>,
    swaps: Option<Arc<AtomicUsize>>,
}

impl Swappable {
    fn with(card: EmulatedDnie) -> Self {
        Self {
            inserted: Arc::new(Mutex::new(Arc::new(Mutex::new(card)))),
            openings: Arc::new(AtomicUsize::new(0)),
            swaps: Some(Arc::new(AtomicUsize::new(0))),
        }
    }

    fn unheard(card: EmulatedDnie) -> Self {
        Self {
            swaps: None,
            ..Self::with(card)
        }
    }

    fn inserted(&self) -> MutexGuard<'_, Arc<Mutex<EmulatedDnie>>> {
        self.inserted.lock().expect("sin envenenar")
    }

    fn swap_for(&self, card: EmulatedDnie) {
        let mut inserted = self.inserted();
        inserted.lock().expect("sin envenenar").remove_card();
        *inserted = Arc::new(Mutex::new(card));
        if let Some(swaps) = &self.swaps {
            swaps.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn openings(&self) -> usize {
        self.openings.load(Ordering::SeqCst)
    }
}

struct Contact(Arc<Mutex<EmulatedDnie>>);

impl Contact {
    fn card(&self) -> MutexGuard<'_, EmulatedDnie> {
        self.0.lock().expect("sin envenenar")
    }
}

impl Transport for Contact {
    fn transmit_raw(&mut self, apdu: &[u8]) -> Result<Vec<u8>, rdnie::Error> {
        self.card().transmit_raw(apdu)
    }

    fn atr(&self) -> Result<Vec<u8>, rdnie::Error> {
        self.card().atr()
    }

    fn reset(&mut self) -> Result<Vec<u8>, rdnie::Error> {
        self.card().reset()
    }

    fn transaction<R>(
        &mut self,
        body: impl FnOnce(&mut dyn Transport) -> Result<R, rdnie::Error>,
    ) -> Result<R, rdnie::Error> {
        self.card().transaction(body)
    }
}

impl CardOpener for Swappable {
    type Card = Contact;

    fn open(&self) -> Result<Session<Contact>, rdnie::Error> {
        self.openings.fetch_add(1, Ordering::SeqCst);
        let card = Arc::clone(&self.inserted());
        let root = card.lock().expect("sin envenenar").component_root();
        Session::open_trusting(Contact(card), root)
    }

    fn insertion(&self) -> Option<u64> {
        let swaps = self.swaps.as_ref()?.load(Ordering::SeqCst);
        Some(u64::try_from(swaps).expect("cabe"))
    }
}

fn a_card_of(holder: &str) -> (EmulatedDnie, Vec<u8>) {
    let mut police = OfThePolice::new();
    police.signing = a_dnie_der(holder, true);
    let der = police.signing.clone();
    (police.on(EmulatedDnie::dnie_3()), der)
}

#[test]
fn a_signature_after_the_card_is_taken_out_fails_and_the_next_operation_opens_a_new_session() {
    let (first, _) = a_card_of("ADA LOVELACE");
    let (second, second_der) = a_card_of("GRACE HOPPER");
    let reader = Swappable::with(first);
    let dnie = DnieToken::new(reader.clone());
    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");

    reader.swap_for(second);
    let failure = signed_by(&dnie).expect_err("la tarjeta se ha ido");
    let found = dnie
        .list(&dnie_store())
        .expect("la otra tarjeta deberia listarse");

    assert_eq!(failure.situation(), Situation::TokenAbsent);
    assert_eq!(reader.openings(), 2);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].der(), second_der.as_slice());
}

#[test]
fn a_listing_after_another_card_goes_in_shows_the_certificate_of_that_card() {
    let (first, _) = a_card_of("ADA LOVELACE");
    let (second, second_der) = a_card_of("GRACE HOPPER");
    let reader = Swappable::with(first);
    let dnie = DnieToken::new(reader.clone());
    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");

    reader.swap_for(second);
    let found = dnie
        .list(&dnie_store())
        .expect("la otra tarjeta deberia listarse");

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].der(), second_der.as_slice());
}

#[test]
fn a_listing_that_needs_the_card_after_an_unheard_swap_reads_the_card_now_in_the_reader() {
    let (first, _) = a_card_of("ADA LOVELACE");
    let (second, second_der) = a_card_of("GRACE HOPPER");
    let reader = Swappable::unheard(first);
    let dnie = DnieToken::new(reader.clone());
    dnie.list(&dnie_store()).expect("el DNIe deberia listarse");

    reader.swap_for(second);
    let every = dnie
        .every_certificate(&dnie_store())
        .expect("la otra tarjeta deberia listarse");

    assert_eq!(reader.openings(), 2);
    assert!(every
        .iter()
        .any(|certificate| certificate.der() == second_der.as_slice()));
}
