//! Prueba de grada C: un PDF firmado con la tarjeta falsa, con el adaptador PKCS#11 de verdad y la postfirma (ADR-0014).

#![cfg(not(windows))]

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use fake_pkcs11::FakeCard;
use rfirma_lib::desktop::adapters::command_line_ports::NativeFilter;
use rfirma_lib::desktop::ports::CertificateFilter;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::application::certificates::certificates_with_their_chains;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};
use rfirma_lib::signing::domain::document_signatures::Validity;
use rfirma_lib::site::domain::protocol::site_filter;

use support::{a_cycle_signed_by, a_one_page_pdf, bridge};

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";
const AUTHENTICATION_CERTIFICATE: &str = "CertAutenticacion";

fn signing_certificate_of(card: &FakeCard) -> TokenCertificate {
    pkcs11::list_certificates_across(&[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse")
        .into_iter()
        .find(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE)
        .expect("la tarjeta falsa lleva el certificado de firma")
}

/// Lo que el filtro del original deja de la tarjeta, el mismo motor para una sede y para la línea de órdenes.
fn offered_under(card: &FakeCard, expression: &str) -> Vec<String> {
    let candidates = certificates_with_their_chains(&RealToken, &[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse");
    let filter = site_filter(&[("filters".to_owned(), expression.to_owned())]);
    let mut labels: Vec<String> = NativeFilter
        .accepted(&filter, candidates)
        .expect("el motor de filtros debería contestar")
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    labels.sort();
    labels
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn the_site_filters_choose_between_the_two_certificates_of_the_dnie_as_autofirma_does() {
    let card = FakeCard::new().expect("la tarjeta falsa debería montarse");

    assert_eq!(offered_under(&card, "signingcert:"), [SIGNING_CERTIFICATE]);
    assert_eq!(
        offered_under(&card, "authcert:"),
        [AUTHENTICATION_CERTIFICATE]
    );
    assert_eq!(offered_under(&card, "dnie:"), [SIGNING_CERTIFICATE]);
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
