//! Prueba de grada C de la validez de cada `ds:Signature` de un XAdES y de una FacturaE, contra el puente real (ADR-0043).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::signing::domain::document_signatures::{
    DocumentSignature, DocumentSignatures, Validity, ValidityReason,
};

use support::bridge;

fn report_of(document: &[u8]) -> DocumentSignatures {
    bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(document))
        .expect("el puente debería leer las firmas")
}

fn testdata(directory: &str, name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata")
        .join(directory)
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn reference(name: &str) -> Vec<u8> {
    testdata("reference", name)
}

fn with_replaced(document: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8(document.to_vec()).expect("la muestra es UTF-8");
    assert_eq!(
        text.matches(from).count(),
        1,
        "«{from}» tiene que estar una vez"
    );
    text.replace(from, to).into_bytes()
}

fn every_signature(signatures: &[DocumentSignature]) -> Vec<&DocumentSignature> {
    signatures
        .iter()
        .flat_map(|signature| {
            std::iter::once(signature).chain(every_signature(&signature.countersignatures))
        })
        .collect()
}

fn assert_valid(signature: &DocumentSignature) {
    assert_eq!(signature.validity, Validity::Valid, "{signature:?}");
    assert_eq!(signature.validity_reason, None, "{signature:?}");
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn every_signature_of_a_countersigned_xades_carries_its_validity() {
    let report = report_of(&reference("xades-enveloping.countersign-tree.xml"));

    let signatures = every_signature(report.signatures());
    assert_eq!(signatures.len(), 2, "la firma y su contrafirma");
    assert_eq!(report.signatures()[0].countersignatures.len(), 1);
    signatures.into_iter().for_each(assert_valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn every_signature_of_a_cosigned_xades_carries_its_validity() {
    let report = report_of(&reference("xades-enveloping.cosign.xml"));

    assert_eq!(report.count(), 2);
    report.signatures().iter().for_each(assert_valid);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn a_facturae_signature_carries_its_validity() {
    let report = report_of(&reference("facturae.xsig"));

    assert_eq!(report.count(), 1);
    assert_valid(&report.signatures()[0]);
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn an_expired_ca_in_the_key_info_makes_the_signature_expired_naming_it() {
    let report = report_of(&testdata("previous-signatures", "xades-expired-ca.xml"));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Expired);
    assert!(
        matches!(
            &signature.validity_reason,
            Some(ValidityReason::CertificateExpired { date, holder: Some(holder) })
                if date.starts_with("2015-") && holder == "rfirma CA caducada de pruebas"
        ),
        "{:?}",
        signature.validity_reason
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn an_altered_xades_is_invalid_and_modified_after_signing() {
    let report = report_of(&with_replaced(
        &reference("xades-enveloping.xml"),
        "Contenido determinista",
        "Contenido alterado",
    ));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn an_altered_facturae_is_invalid_and_modified_after_signing() {
    let report = report_of(&with_replaced(
        &reference("facturae.xsig"),
        "<BatchIdentifier>RFIRMA-0001</BatchIdentifier>",
        "<BatchIdentifier>RFIRMA-0002</BatchIdentifier>",
    ));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
}

#[test]
#[ignore = "grada C: necesita librfirma_crypto.so (just test-native)"]
fn altering_what_a_signature_signs_leaves_its_countersignature_valid() {
    let report = report_of(&with_replaced(
        &reference("xades-enveloping.countersign-tree.xml"),
        "Contenido determinista",
        "Contenido alterado",
    ));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
    assert_valid(&signature.countersignatures[0]);
}
