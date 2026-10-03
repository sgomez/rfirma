use super::*;
use crate::signing::domain::document_signatures::{
    DocumentFinding, DocumentSignature, SignatureStatus, Validity, ValidityReason,
};

#[test]
fn an_unsigned_pdf_reports_no_previous_signatures() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[],"changedAfterLastSignature":false,"findings":[]}"#,
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
            "reason":null,
            "validity":"valid"
        }],"changedAfterLastSignature":false,"findings":[]}"#,
    )
    .expect("es valida");

    assert_eq!(
        report.signatures(),
        [DocumentSignature {
            name: "LOVELACE BYRON ADA".to_owned(),
            id_number: "IDCES-00000000T".to_owned(),
            organization_identifier: Some("VATES-A00000000".to_owned()),
            organization_name: Some("FNMT-RCM".to_owned()),
            issuer: "AC FNMT Usuarios".to_owned(),
            certificate_serial_number: "1234567890".to_owned(),
            signing_time: Some("2024-01-01T10:00:00Z".to_owned()),
            status: Some(SignatureStatus::Valid),
            reason: None,
            validity: Validity::Valid,
            validity_reason: None,
            countersignatures: Vec::new(),
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
            "reason":null,
            "validity":"valid"
        }],"changedAfterLastSignature":false,"findings":[]}"#,
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
            "reason":"CERTIFICATE_EXPIRED",
            "validity":"valid"
        }],"changedAfterLastSignature":true,"findings":[]}"#,
    )
    .expect("es valida");

    assert_eq!(
        report.signatures()[0].status,
        Some(SignatureStatus::CertificateExpired)
    );
    assert_eq!(
        report.signatures()[0].reason.as_deref(),
        Some("CERTIFICATE_EXPIRED")
    );
    assert!(report.changed_after_last_signature());
}

#[test]
fn a_signature_the_bridge_did_not_validate_crosses_without_a_status_and_with_its_countersignatures()
{
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[{
            "subject":"CN=LOVELACE BYRON ADA",
            "issuer":"CN=AC FNMT Usuarios",
            "serialNumber":"1",
            "signingTime":null,
            "status":null,
            "reason":null,
            "validity":"valid",
            "countersignatures":[{
                "subject":"CN=BABBAGE CHARLES",
                "issuer":"CN=AC FNMT Usuarios",
                "serialNumber":"2",
                "signingTime":null,
                "status":null,
                "reason":null,
                "validity":"valid",
                "countersignatures":[]
            }]
        }],"changedAfterLastSignature":false,"findings":[]}"#,
    )
    .expect("es valida");

    let signature = &report.signatures()[0];
    assert_eq!(signature.status, None);
    assert_eq!(signature.countersignatures.len(), 1);
    assert_eq!(signature.countersignatures[0].name, "BABBAGE CHARLES");
    assert_eq!(report.warning_count(), 0);
}

#[test]
fn a_previous_signatures_answer_missing_the_list_is_a_malformed_answer() {
    assert!(parse_previous_signatures(
        r#"{"ok":true,"changedAfterLastSignature":false,"findings":[]}"#
    )
    .is_err());
}

#[test]
fn a_previous_signatures_answer_missing_the_changed_flag_is_a_malformed_answer() {
    assert!(parse_previous_signatures(r#"{"ok":true,"signatures":[],"findings":[]}"#).is_err());
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
            "reason":null,
            "validity":"valid"
        }],"changedAfterLastSignature":false,"findings":[]}"#
    )
    .is_err());
}

fn a_report_with_one_signature(validity: &str) -> String {
    format!(
        r#"{{"ok":true,"signatures":[{{
            "subject":"CN=LOVELACE BYRON ADA",
            "issuer":"CN=AC FNMT Usuarios",
            "serialNumber":"1",
            "signingTime":null,
            "status":null,
            "reason":null,
            {validity}
        }}],"changedAfterLastSignature":false,"findings":[]}}"#
    )
}

#[test]
fn a_previous_signature_carries_its_validity_and_the_worst_reason() {
    let expired = parse_previous_signatures(&a_report_with_one_signature(
        r#""validity":"expired","validityReason":{"kind":"certificateExpired",
            "date":"2020-01-01T00:00:00Z","holder":null,"closedBy":null}"#,
    ))
    .expect("es valida");
    let closed = parse_previous_signatures(&a_report_with_one_signature(
        r#""validity":"invalid","validityReason":{"kind":"cosignNotAdmitted",
            "date":null,"holder":null,"closedBy":"CN=BABBAGE CHARLES"}"#,
    ))
    .expect("es valida");

    assert_eq!(expired.signatures()[0].validity, Validity::Expired);
    assert_eq!(
        expired.signatures()[0].validity_reason,
        Some(ValidityReason::CertificateExpired {
            date: "2020-01-01T00:00:00Z".to_owned(),
            holder: None,
        })
    );
    assert_eq!(closed.signatures()[0].validity, Validity::Invalid);
    assert_eq!(
        closed.signatures()[0].validity_reason,
        Some(ValidityReason::CosignNotAdmitted {
            closed_by: Some("CN=BABBAGE CHARLES".to_owned()),
        })
    );
}

#[test]
fn every_other_reason_crosses_by_its_name() {
    let reasons = [
        ("modifiedAfterSigning", ValidityReason::ModifiedAfterSigning),
        ("damaged", ValidityReason::Damaged),
        ("unknownSignatureType", ValidityReason::UnknownSignatureType),
        (
            "certificateNotYetValid",
            ValidityReason::CertificateNotYetValid {
                date: "2030-01-01T00:00:00Z".to_owned(),
            },
        ),
    ];

    for (kind, expected) in reasons {
        let report = parse_previous_signatures(&a_report_with_one_signature(&format!(
            r#""validity":"invalid","validityReason":{{"kind":"{kind}",
                "date":"2030-01-01T00:00:00Z","holder":null,"closedBy":null}}"#
        )))
        .expect("es valida");
        assert_eq!(report.signatures()[0].validity_reason, Some(expected));
    }
}

#[test]
fn the_findings_of_the_document_cross_apart_from_the_signatures() {
    let report = parse_previous_signatures(
        r#"{"ok":true,"signatures":[],"changedAfterLastSignature":true,
            "findings":["modifiedAfterLastSignature","formFilledAfterSigning","contentAddedOnTop"]}"#,
    )
    .expect("es valida");

    assert_eq!(
        report.findings(),
        [
            DocumentFinding::ModifiedAfterLastSignature,
            DocumentFinding::FormFilledAfterSigning,
            DocumentFinding::ContentAddedOnTop,
        ]
    );
}

#[test]
fn a_previous_signature_without_a_validity_is_a_malformed_answer() {
    assert!(
        parse_previous_signatures(&a_report_with_one_signature(r#""validityReason":null"#))
            .is_err()
    );
}

#[test]
fn a_validity_a_reason_or_a_finding_this_binary_does_not_know_is_a_malformed_answer() {
    assert!(
        parse_previous_signatures(&a_report_with_one_signature(r#""validity":"quiza""#)).is_err()
    );
    assert!(parse_previous_signatures(&a_report_with_one_signature(
        r#""validity":"invalid","validityReason":{"kind":"quiza"}"#
    ))
    .is_err());
    assert!(parse_previous_signatures(
        r#"{"ok":true,"signatures":[],"changedAfterLastSignature":false,"findings":["quiza"]}"#
    )
    .is_err());
}

#[test]
fn a_previous_signatures_answer_missing_the_findings_is_a_malformed_answer() {
    assert!(parse_previous_signatures(
        r#"{"ok":true,"signatures":[],"changedAfterLastSignature":false}"#
    )
    .is_err());
}
