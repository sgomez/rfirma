use super::*;
use crate::signing::domain::previous_signatures::{PreviousSignature, SignatureStatus};

#[test]
fn an_unsigned_pdf_reports_no_previous_signatures() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[],"changedAfterLastSignature":false}"#,
    )
    .expect("es valida");

    assert_eq!(report.count(), 0);
    assert!(!report.changed_after_last_signature());
}

#[test]
fn a_previous_signature_translates_the_subject_and_the_issuer_with_the_holder_utilities() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[{
            "subject":"CN=LOVELACE BYRON ADA, SERIALNUMBER=IDCES-00000000T, organizationIdentifier=VATES-A00000000, O=FNMT-RCM",
            "issuer":"CN=AC FNMT Usuarios, OU=Ceres, O=FNMT-RCM, C=ES",
            "serialNumber":"1234567890",
            "signingTime":"2024-01-01T10:00:00Z",
            "status":"valid",
            "reason":null
        }],"changedAfterLastSignature":false}"#,
    )
    .expect("es valida");

    assert_eq!(
        report.signatures(),
        [PreviousSignature {
            name: "LOVELACE BYRON ADA".to_owned(),
            id_number: "IDCES-00000000T".to_owned(),
            organization_identifier: Some("VATES-A00000000".to_owned()),
            issuer: "AC FNMT Usuarios".to_owned(),
            certificate_serial_number: "1234567890".to_owned(),
            signing_time: Some("2024-01-01T10:00:00Z".to_owned()),
            status: SignatureStatus::Valid,
            reason: None,
        }]
    );
}

#[test]
fn a_previous_signature_without_a_signing_time_crosses_as_nothing_and_not_a_failure() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[{
            "subject":"CN=LOVELACE BYRON ADA",
            "issuer":"CN=AC FNMT Usuarios",
            "serialNumber":"1",
            "signingTime":null,
            "status":"valid",
            "reason":null
        }],"changedAfterLastSignature":false}"#,
    )
    .expect("es valida");

    assert_eq!(report.signatures()[0].signing_time, None);
}

#[test]
fn a_previous_signature_carries_its_status_and_the_reason_of_the_original() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[{
            "subject":"CN=LOVELACE BYRON ADA",
            "issuer":"CN=AC FNMT Usuarios",
            "serialNumber":"1",
            "signingTime":null,
            "status":"certificateExpired",
            "reason":"CERTIFICATE_EXPIRED"
        }],"changedAfterLastSignature":true}"#,
    )
    .expect("es valida");

    assert_eq!(
        report.signatures()[0].status,
        SignatureStatus::CertificateExpired
    );
    assert_eq!(
        report.signatures()[0].reason.as_deref(),
        Some("CERTIFICATE_EXPIRED")
    );
    assert!(report.changed_after_last_signature());
}

#[test]
fn a_previous_signatures_answer_missing_the_list_is_a_malformed_answer() {
    assert!(parse_previous_signatures(r#"{"ok":true,"changedAfterLastSignature":false}"#).is_err());
}

#[test]
fn a_previous_signatures_answer_missing_the_changed_flag_is_a_malformed_answer() {
    assert!(parse_previous_signatures(r#"{"ok":true,"signatures":[]}"#).is_err());
}

#[test]
fn a_previous_signature_with_a_status_this_binary_does_not_know_is_a_malformed_answer() {
    assert!(parse_previous_signatures(
        r#"{"ok":true,"signatures":[{
            "subject":"CN=LOVELACE BYRON ADA",
            "issuer":"CN=AC FNMT Usuarios",
            "serialNumber":"1",
            "signingTime":null,
            "status":"quiza",
            "reason":null
        }],"changedAfterLastSignature":false}"#
    )
    .is_err());
}
