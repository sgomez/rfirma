use super::{PreviousSignature, PreviousSignaturesReport, SignatureStatus, Tone};

fn a_previous_signature_with_status(status: SignatureStatus) -> PreviousSignature {
    PreviousSignature {
        name: "LOVELACE BYRON ADA".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        organization_identifier: None,
        issuer: "AC FNMT Usuarios".to_owned(),
        certificate_serial_number: "1".to_owned(),
        signing_time: Some("2024-01-01T10:00:00Z".to_owned()),
        status,
        reason: None,
    }
}

#[test]
fn are_ko_the_certificate_expired_not_yet_valid_broken_and_unverifiable_statuses() {
    let ko = [
        SignatureStatus::CertificateExpired,
        SignatureStatus::CertificateNotYetValid,
        SignatureStatus::Broken,
        SignatureStatus::Unverifiable,
    ];
    for status in ko {
        assert!(status.is_ko(), "{status:?} debería ser KO");
    }
}

#[test]
fn are_not_ko_the_valid_and_not_fully_checked_statuses() {
    let not_ko = [SignatureStatus::Valid, SignatureStatus::NotFullyChecked];
    for status in not_ko {
        assert!(!status.is_ko(), "{status:?} no debería ser KO");
    }
}

#[test]
fn a_report_with_every_signature_valid_and_no_change_has_no_warnings_and_is_informational() {
    let report = PreviousSignaturesReport::new(
        vec![a_previous_signature_with_status(SignatureStatus::Valid)],
        false,
    );

    assert_eq!(report.warning_count(), 0);
    assert_eq!(report.tone(), Tone::Information);
}

#[test]
fn a_report_with_a_signature_not_fully_checked_and_no_change_warns_once_and_is_indeterminate() {
    let report = PreviousSignaturesReport::new(
        vec![a_previous_signature_with_status(
            SignatureStatus::NotFullyChecked,
        )],
        false,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Indeterminate);
}

#[test]
fn a_report_with_a_ko_signature_warns_once_and_is_of_attention() {
    let report = PreviousSignaturesReport::new(
        vec![a_previous_signature_with_status(SignatureStatus::Broken)],
        false,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn a_document_changed_after_the_last_signature_warns_once_and_is_of_attention_on_its_own() {
    let report = PreviousSignaturesReport::new(
        vec![a_previous_signature_with_status(SignatureStatus::Valid)],
        true,
    );

    assert_eq!(report.warning_count(), 1);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn ko_not_fully_checked_and_a_change_all_add_up_and_attention_wins() {
    let report = PreviousSignaturesReport::new(
        vec![
            a_previous_signature_with_status(SignatureStatus::CertificateExpired),
            a_previous_signature_with_status(SignatureStatus::NotFullyChecked),
        ],
        true,
    );

    assert_eq!(report.warning_count(), 3);
    assert_eq!(report.tone(), Tone::Attention);
}

#[test]
fn the_tone_order_is_information_then_indeterminate_then_attention() {
    assert!(Tone::Information < Tone::Indeterminate);
    assert!(Tone::Indeterminate < Tone::Attention);
}
