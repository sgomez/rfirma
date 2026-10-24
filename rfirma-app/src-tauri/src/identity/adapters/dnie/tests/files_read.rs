use rdnie_emulator::{EmulatedDnie, DNIE_4_ATR};

use super::super::{dnie_store, seen_dnie};
use super::in_the_reader::{the_pin, the_signing_certificate, InTheReader};
use super::OfThePolice;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::seen_dnie::DnieInTheReader;
use crate::identity::ports::Token;

const AUTHENTICATION_FILE: u8 = 1;
const SIGNING_FILE: u8 = 2;
const INTERMEDIATE_FILE: u8 = 3;

fn in_the_reader() -> InTheReader {
    InTheReader::new(OfThePolice::new().on(EmulatedDnie::dnie_4()))
}

fn certificate_files_read(reader: &InTheReader) -> Vec<u8> {
    let mut files: Vec<u8> = reader
        .card()
        .selected()
        .iter()
        .filter_map(|path| path.last())
        .filter(|file| file[0] == 0x70)
        .map(|file| file[1])
        .collect();
    files.sort_unstable();
    files.dedup();
    files
}

#[test]
fn a_signature_reads_no_certificate_file_of_the_card() {
    let reader = in_the_reader();

    reader
        .token()
        .sign_with_secret(
            &the_signing_certificate(),
            &the_pin(),
            SignatureAlgorithm::Sha256Rsa,
            b"datos",
        )
        .expect("el DNIe deberia firmar");

    assert_eq!(certificate_files_read(&reader), Vec::<u8>::new());
}

#[test]
fn a_listing_reads_the_signing_certificate_file_and_no_other() {
    let reader = in_the_reader();

    reader
        .token()
        .list(&dnie_store())
        .expect("el DNIe deberia listarse");

    assert_eq!(certificate_files_read(&reader), vec![SIGNING_FILE]);
}

#[test]
fn every_certificate_reads_the_three_files() {
    let reader = in_the_reader();

    reader
        .token()
        .every_certificate(&dnie_store())
        .expect("el DNIe deberia listarse");

    assert_eq!(
        certificate_files_read(&reader),
        vec![AUTHENTICATION_FILE, SIGNING_FILE, INTERMEDIATE_FILE]
    );
}

#[test]
fn the_debug_report_does_not_reset_the_card_and_gives_its_atr() {
    let reader = in_the_reader();
    let atr: String = DNIE_4_ATR
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect();

    let DnieInTheReader::Seen(seen) = seen_dnie(&reader) else {
        panic!("hay un DNIe");
    };

    assert_eq!(seen.atr, atr);
    assert_eq!(seen.version, "4.0");
    assert_eq!(reader.card().resets(), 0);
}
