//! Prueba de grada C: un PDF firmado con la tarjeta falsa, con el adaptador PKCS#11 de verdad y la postfirma (ADR-0014).

#![cfg(not(windows))]

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use fake_pkcs11::FakeCard;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::document_signatures::Validity;

use support::{a_cycle_signed_by, a_one_page_pdf, bridge};

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";

fn signing_certificate_of(card: &FakeCard) -> TokenCertificate {
    pkcs11::list_certificates_across(&[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse")
        .into_iter()
        .find(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE)
        .expect("la tarjeta falsa lleva el certificado de firma")
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_pdf_signed_with_the_fake_card_is_valid_and_the_card_saw_the_pin_once() {
    let card = FakeCard::new().expect("la tarjeta falsa debería montarse");
    let certificate = signing_certificate_of(&card);

    let signed = a_cycle_signed_by(
        &certificate,
        FakeCard::PIN,
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );

    let report = bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(&signed))
        .expect("el puente debería leer las firmas");
    assert_eq!(report.count(), 1);
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
    assert_eq!(
        card.calls_to("C_Login").len(),
        1,
        "{:?}",
        card.calls_to("C_Login")
    );
    assert!(!card.calls_to("C_Sign").is_empty(), "{:?}", card.calls());
}
