//! Prueba de grada C de la validez de cada SignerInfo de un CAdES, contrafirmas incluidas, contra el puente real (ADR-0043).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::signing::domain::bridge::SignatureOperation;
use rfirma_lib::signing::domain::document_signatures::{
    DocumentSignature, DocumentSignatures, Validity, ValidityReason,
};

use support::{bridge, cades_cycle, sign_cades};

const SIGNED_CONTENT: &[u8] = b"rfirma: contenido firmado en CAdES\n";

fn report_of(signature: &[u8]) -> DocumentSignatures {
    bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(signature))
        .expect("el puente debería leer las firmas")
}

fn sample(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../testdata/previous-signatures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn with_the_signed_content_altered(signature: &[u8]) -> Vec<u8> {
    let at = signature
        .windows(SIGNED_CONTENT.len())
        .position(|window| window == SIGNED_CONTENT)
        .expect("la muestra lleva el contenido dentro");
    let mut altered = signature.to_vec();
    altered[at] = b'R';
    altered
}

fn assert_expired_in_2020(signature: &DocumentSignature) {
    assert_eq!(signature.validity, Validity::Expired);
    assert!(
        matches!(
            &signature.validity_reason,
            Some(ValidityReason::CertificateExpired { date, holder: None }) if date.starts_with("2020-")
        ),
        "{:?}",
        signature.validity_reason
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_freshly_signed_cades_has_a_valid_signature() {
    let report = report_of(&sign_cades(SIGNED_CONTENT, "implicit"));

    assert_eq!(report.count(), 1);
    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Valid);
    assert_eq!(signature.validity_reason, None);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn an_intact_cades_with_its_certificate_expired_is_expired_with_the_date() {
    let report = report_of(&sample("cades-expired.csig"));

    assert_eq!(report.count(), 1);
    assert_expired_in_2020(&report.signatures()[0]);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn an_altered_cades_with_its_certificate_expired_is_invalid_and_modified_after_signing() {
    let report = report_of(&with_the_signed_content_altered(&sample(
        "cades-expired.csig",
    )));

    assert_eq!(report.count(), 1);
    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn each_countersignature_carries_its_own_validity() {
    let report = report_of(&sample("cades-countersigned-by-expired.csig"));

    let signer = &report.signatures()[0];
    assert_eq!(signer.validity, Validity::Valid);
    assert_eq!(signer.validity_reason, None);
    assert_eq!(signer.countersignatures.len(), 1);
    assert_expired_in_2020(&signer.countersignatures[0]);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_countersignature_made_with_the_token_is_valid_like_its_signer() {
    let signed = sign_cades(SIGNED_CONTENT, "implicit");
    let countersigned = cades_cycle(
        &signed,
        SignatureOperation::Countersign,
        &[("target", "tree")],
    );

    let report = report_of(&countersigned);

    let signer = &report.signatures()[0];
    assert_eq!(signer.validity, Validity::Valid);
    assert_eq!(signer.countersignatures.len(), 1);
    assert_eq!(signer.countersignatures[0].validity, Validity::Valid);
    assert_eq!(signer.countersignatures[0].validity_reason, None);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn an_altered_cades_with_its_certificate_in_force_is_invalid_and_modified_after_signing() {
    let report = report_of(&with_the_signed_content_altered(&sign_cades(
        SIGNED_CONTENT,
        "implicit",
    )));

    let signature = &report.signatures()[0];
    assert_eq!(signature.validity, Validity::Invalid);
    assert_eq!(
        signature.validity_reason,
        Some(ValidityReason::ModifiedAfterSigning)
    );
}
