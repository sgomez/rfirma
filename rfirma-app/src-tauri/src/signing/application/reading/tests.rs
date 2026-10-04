use super::signatures_of;
use crate::signing::application::tests::AnEngineThatReports;
use crate::signing::domain::{DocumentSignature, DocumentSignatures, SignatureStandard, Validity};
use base64::Engine;

#[test]
fn signatures_of_sends_the_document_as_base_64_to_the_engine() {
    let document: &[u8] = b"%PDF-1.7 contenido";
    let engine = AnEngineThatReports::default();

    signatures_of(document, &engine).expect("el motor contesta");

    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(engine.last_document_b64())
            .expect("es base64"),
        b"%PDF-1.7 contenido"
    );
}

#[test]
fn signatures_of_returns_what_the_engine_reports() {
    let document: &[u8] = b"%PDF-1.7 contenido";
    let signature = DocumentSignature {
        name: "LOVELACE BYRON ADA".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        organization_identifier: None,
        organization_name: None,
        issuer: "AC FNMT Usuarios".to_owned(),
        certificate_subject: "CN=FIRMANTE".to_owned(),
        certificate_issuer: "CN=AC FNMT Usuarios, O=FNMT-RCM, C=ES".to_owned(),
        certificate_serial_number: "1".to_owned(),
        certificate_valid_from: None,
        certificate_valid_until: None,
        signature_algorithm: None,
        profile: None,
        signing_time: Some("2024-01-01T10:00:00Z".to_owned()),
        validity: Validity::Valid,
        validity_reason: None,
        signing_date: None,
        closes_document: false,
        countersignatures: Vec::new(),
    };
    let engine = AnEngineThatReports::default()
        .answering(DocumentSignatures::new(vec![signature.clone()], false));

    let report = signatures_of(document, &engine).expect("el motor contesta");

    assert_eq!(report.signatures(), [signature]);
}

#[test]
fn the_signatures_of_a_pdf_are_told_as_pades() {
    let document: &[u8] = b"%PDF-1.7 contenido";
    let engine = AnEngineThatReports::default();

    let report = signatures_of(document, &engine).expect("el motor contesta");

    assert_eq!(report.format(), SignatureStandard::Pades);
}

#[test]
fn a_cades_reaches_the_engine_and_is_told_as_cades() {
    let cades = [
        0x30, 0x80, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02, 0xa0, 0x80,
    ];
    let document = &cades[..];
    let engine = AnEngineThatReports::default();

    let report = signatures_of(document, &engine).expect("el motor contesta");

    assert_eq!(report.format(), SignatureStandard::Cades);
    assert!(engine.was_asked());
}

#[test]
fn a_xades_reaches_the_engine_and_is_told_as_xades() {
    let document: &[u8] = b"<?xml version=\"1.0\"?><a/>";
    let engine = AnEngineThatReports::default();

    let report = signatures_of(document, &engine).expect("el motor contesta");

    assert_eq!(report.format(), SignatureStandard::Xades);
    assert!(engine.was_asked());
}

#[test]
fn a_file_of_an_unrecognized_format_has_no_signatures_and_does_not_reach_the_engine() {
    let document: &[u8] = b"\x89PNG\r\n\x1a\n datos";
    let engine = AnEngineThatReports::default();

    let report = signatures_of(document, &engine).expect("no hace falta el motor");

    assert_eq!(report.format(), SignatureStandard::Unrecognized);
    assert_eq!(report.count(), 0);
    assert!(!engine.was_asked());
}

#[test]
fn the_signatures_of_a_certified_pdf_reach_the_engine() {
    let document: &[u8] = b"%PDF-1.7\n9 0 obj\n<< /Type /Sig /Reference [ << /TransformMethod /DocMDP >> ] >>\nendobj";
    let engine = AnEngineThatReports::default();

    signatures_of(document, &engine).expect("leer las firmas no las rompe");

    assert!(engine.was_asked());
}

#[test]
fn the_signatures_of_an_encrypted_pdf_are_not_read() {
    let document: &[u8] = b"%PDF-1.7\ntrailer\n<< /Root 1 0 R /Encrypt 5 0 R >>";
    let engine = AnEngineThatReports::default();

    assert!(signatures_of(document, &engine).is_err());
    assert!(!engine.was_asked());
}
