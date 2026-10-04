use super::{DocumentSignature, DocumentSignatures, Tone, Validity};

fn a_previous_signature_with_validity(validity: Validity) -> DocumentSignature {
    DocumentSignature {
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
        validity,
        validity_reason: None,
        signing_date: None,
        closes_document: false,
        countersignatures: Vec::new(),
    }
}

#[test]
fn a_report_with_every_signature_valid_and_no_change_has_no_warnings_and_is_informational() {
    let report = DocumentSignatures::new(
        vec![a_previous_signature_with_validity(Validity::Valid)],
        false,
    );

    assert_eq!(report.warning_count(), 0);
    assert_eq!(report.tone(), Tone::Information);
}

#[test]
fn a_report_with_an_expired_signature_warns_once_and_is_of_attention() {
    let report = DocumentSignatures::new(
        vec![a_previous_signature_with_validity(Validity::Expired)],
        false,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn a_report_with_an_invalid_signature_warns_once_and_is_of_attention() {
    let report = DocumentSignatures::new(
        vec![a_previous_signature_with_validity(Validity::Invalid)],
        false,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn a_document_changed_after_the_last_signature_warns_once_and_is_of_attention_on_its_own() {
    let report = DocumentSignatures::new(
        vec![a_previous_signature_with_validity(Validity::Valid)],
        true,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn bad_signatures_and_a_change_all_add_up() {
    let report = DocumentSignatures::new(
        vec![
            a_previous_signature_with_validity(Validity::Expired),
            a_previous_signature_with_validity(Validity::Invalid),
        ],
        true,
    );

    assert_eq!(report.warning_count(), 3);
    assert_eq!(report.tone(), Tone::Attention);
}
