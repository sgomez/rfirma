//! Prueba de grada C del informe de firmas previas contra el puente real (ADR-0014).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::signing::application::cycle::ALGORITHM;
use rfirma_lib::signing::domain::bridge::{Format, SignatureOperation};

use support::{a_cycle_of, a_one_page_pdf, bridge};

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
}
