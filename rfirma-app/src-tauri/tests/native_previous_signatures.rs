//! Prueba de grada C del informe de firmas previas contra el puente real (ADR-0014, TD-100).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation, SignatureStatus, Tone};

use support::{a_cycle_of, a_one_page_pdf, bridge};

/// La version del encabezado entra en el `/ByteRange`: el resumen de la firma deja de cuadrar.
fn with_the_signed_bytes_altered(pdf: &[u8]) -> Vec<u8> {
    const HEADER: &[u8] = b"%PDF-1.";

    let at = pdf
        .windows(HEADER.len())
        .position(|window| window == HEADER)
        .expect("el encabezado tiene que estar")
        + HEADER.len();
    let mut altered = pdf.to_vec();
    altered[at] = if altered[at] == b'7' { b'4' } else { b'7' };
    altered
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn the_bridge_reports_the_signer_and_the_signing_time_of_a_pades_signature() {
    let bridge = bridge();
    let signed = a_cycle_of(
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );
    let document_b64 = base64::engine::general_purpose::STANDARD.encode(&signed);

    let report = bridge.previous_signatures(&document_b64).unwrap();

    assert_eq!(report.count(), 1);
    let signature = &report.signatures()[0];
    assert_eq!(signature.id_number, "IDCES-99999999R");
    assert_eq!(signature.organization_identifier, None);
    assert!(
        signature.signing_time.is_some(),
        "el puente debería devolver el instante de la firma"
    );
    assert_eq!(signature.status, SignatureStatus::Valid);
    assert_eq!(report.warning_count(), 0);
    assert_eq!(report.tone(), Tone::Information);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_pades_signature_broken_after_signing_is_ko_and_warns_with_attention() {
    let bridge = bridge();
    let signed = a_cycle_of(
        Format::Pades,
        ALGORITHM,
        &a_one_page_pdf(),
        SignatureOperation::Sign,
        &[],
    );
    let altered = with_the_signed_bytes_altered(&signed);
    let document_b64 = base64::engine::general_purpose::STANDARD.encode(&altered);

    let report = bridge.previous_signatures(&document_b64).unwrap();

    assert_eq!(report.count(), 1);
    let signature = &report.signatures()[0];
    assert_eq!(signature.status, SignatureStatus::Broken);
    assert!(signature.status.is_ko());
    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}
