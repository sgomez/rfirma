use serde_json::{json, Value};

use super::*;
use crate::desktop::application::command_line::tests::schema::conforming_json;

fn in_json_over(
    words: &[&str],
    files: &dyn CommandLineFiles,
    engine: &dyn PreviousSignaturesEngine,
) -> Value {
    let verifier = Answering::with(&["Firma valida"]);
    let outcome = attended(words, files, &verifier, engine, &Untouched);
    assert_eq!(outcome.exit_code, SUCCEEDED, "{:?}", outcome.stderr);
    assert!(outcome.stderr.is_empty(), "{:?}", outcome.stderr);
    assert!(verifier.asked.borrow().is_empty());
    conforming_json("verify", &outcome.stdout)
}

fn in_json(signatures: Vec<DocumentSignature>) -> Value {
    in_json_over(
        &["verify", "-i", "firmado.pdf", "-json"],
        &OneFile(A_PDF),
        &Reading(Ok(signatures)),
    )
}

fn a_complete_signature() -> DocumentSignature {
    DocumentSignature {
        organization_identifier: Some("VATES-Q0000000J".to_owned()),
        organization_name: Some("ENTIDAD DE PRUEBAS".to_owned()),
        certificate_subject: "CN=NOMBRE APELLIDO, SERIALNUMBER=IDCES-99999999R".to_owned(),
        certificate_serial_number: "1193046".to_owned(),
        certificate_valid_from: Some("2025-01-01T00:00:00Z".to_owned()),
        certificate_valid_until: Some("2029-01-01T00:00:00.250Z".to_owned()),
        signature_algorithm: Some("SHA256withRSA".to_owned()),
        profile: Some("AdES-BES".to_owned()),
        signing_date: Some(SigningDate::Declared {
            at: "2026-09-14T10:32:05+02:00".to_owned(),
        }),
        ..a_signature(
            "NOMBRE APELLIDO",
            "IDCES-99999999R",
            Some("2026-09-14T08:32:05Z"),
        )
    }
}

fn with_validity(
    validity: Validity,
    reason: Option<ValidityReason>,
    signature: DocumentSignature,
) -> DocumentSignature {
    DocumentSignature {
        validity,
        validity_reason: reason,
        ..signature
    }
}

fn invalid_because(reason: ValidityReason) -> DocumentSignature {
    with_validity(
        Validity::Invalid,
        Some(reason),
        a_signature("UNA PERSONA", "", None),
    )
}

#[test]
fn verify_json_gives_the_standard_and_each_signature_from_the_previous_signatures_model() {
    let verified = in_json(vec![a_complete_signature()]);

    assert_eq!(
        verified,
        json!({
            "standard": "PAdES",
            "validity": "valid",
            "findings": [],
            "signatures": [{
                "validity": "valid",
                "signer": {
                    "commonName": "NOMBRE APELLIDO",
                    "serialNumber": "IDCES-99999999R",
                    "organizationIdentifier": "VATES-Q0000000J",
                    "organizationName": "ENTIDAD DE PRUEBAS",
                },
                "certificate": {
                    "subject": "CN=NOMBRE APELLIDO, SERIALNUMBER=IDCES-99999999R",
                    "issuer": "CN=AC FNMT Usuarios, O=FNMT-RCM, C=ES",
                    "serialNumber": "123456",
                    "notBefore": "2025-01-01T00:00:00Z",
                    "notAfter": "2029-01-01T00:00:00Z",
                },
                "signatureAlgorithm": {
                    "name": "sha256WithRSAEncryption",
                    "oid": "1.2.840.113549.1.1.11",
                },
                "signingTime": {"at": "2026-09-14T08:32:05Z", "source": "declared"},
                "closesDocument": false,
                "countersignatures": [],
            }],
        })
    );
}

#[test]
fn a_stamped_signature_gives_its_time_as_a_timestamp_with_the_tsa() {
    let stamped = DocumentSignature {
        signing_date: Some(SigningDate::Stamped {
            at: "2026-09-20T16:01:44Z".to_owned(),
            tsa: "TSA DE PRUEBAS".to_owned(),
        }),
        closes_document: true,
        ..a_signature("UNA PERSONA", "", None)
    };

    let signature = &in_json(vec![stamped])["signatures"][0];

    assert_eq!(
        signature["signingTime"],
        json!({"at": "2026-09-20T16:01:44Z", "source": "timestamp", "tsa": "TSA DE PRUEBAS"})
    );
    assert_eq!(signature["closesDocument"], true);
}

#[test]
fn a_signature_with_only_the_bridge_signing_time_gives_it_as_declared() {
    let signature = &in_json(vec![a_signature(
        "UNA PERSONA",
        "",
        Some("2026-09-14T08:32:05Z"),
    )])["signatures"][0];

    assert_eq!(
        signature["signingTime"],
        json!({"at": "2026-09-14T08:32:05Z", "source": "declared"})
    );
}

#[test]
fn an_algorithm_out_of_the_table_goes_with_its_jca_name_and_no_oid() {
    let signature = DocumentSignature {
        signature_algorithm: Some("MD5withRSA".to_owned()),
        ..a_complete_signature()
    };

    let verified = in_json(vec![signature]);

    assert_eq!(
        verified["signatures"][0]["signatureAlgorithm"],
        json!({"name": "MD5withRSA"})
    );
}

#[test]
fn what_a_signature_does_not_say_is_left_out_and_the_profile_never_goes() {
    let signature = &in_json(vec![a_signature("", "", None)])["signatures"][0];

    let mut keys: Vec<&String> = signature.as_object().expect("objeto").keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "certificate",
            "closesDocument",
            "countersignatures",
            "signer",
            "validity"
        ]
    );
    assert_eq!(signature["signer"], json!({}));
}

#[test]
fn every_reason_goes_by_its_type_with_its_fields_in_camel_case() {
    let reasons = [
        (
            ValidityReason::CertificateExpired {
                date: "2026-01-02T01:00:00+01:00".to_owned(),
                holder: Some("AC INTERMEDIA".to_owned()),
            },
            json!({"type": "certificateExpired", "date": "2026-01-02T00:00:00Z", "holder": "AC INTERMEDIA"}),
        ),
        (
            ValidityReason::CertificateExpired {
                date: "2026-01-02T00:00:00Z".to_owned(),
                holder: None,
            },
            json!({"type": "certificateExpired", "date": "2026-01-02T00:00:00Z"}),
        ),
        (
            ValidityReason::CertificateNotYetValid {
                date: "2030-01-01T00:00:00Z".to_owned(),
            },
            json!({"type": "certificateNotYetValid", "date": "2030-01-01T00:00:00Z"}),
        ),
        (
            ValidityReason::ModifiedAfterSigning,
            json!({"type": "modifiedAfterSigning"}),
        ),
        (ValidityReason::Damaged, json!({"type": "damaged"})),
        (
            ValidityReason::UnknownSignatureType,
            json!({"type": "unknownSignatureType"}),
        ),
        (
            ValidityReason::UnsupportedAlgorithm,
            json!({"type": "unsupportedAlgorithm"}),
        ),
        (
            ValidityReason::CosignNotAdmitted {
                closed_by: Some("QUIEN CERTIFICA".to_owned()),
            },
            json!({"type": "cosignNotAdmitted", "closedBy": "QUIEN CERTIFICA"}),
        ),
        (
            ValidityReason::CosignNotAdmitted { closed_by: None },
            json!({"type": "cosignNotAdmitted"}),
        ),
    ];

    for (reason, expected) in reasons {
        let verified = in_json(vec![invalid_because(reason)]);

        assert_eq!(verified["signatures"][0]["validity"], "invalid");
        assert_eq!(verified["signatures"][0]["reason"], expected);
    }
}

#[test]
fn the_document_validity_is_the_worst_of_all_signatures_countersignatures_included() {
    let expired = with_validity(
        Validity::Expired,
        Some(ValidityReason::CertificateExpired {
            date: "2026-01-02T00:00:00Z".to_owned(),
            holder: None,
        }),
        a_signature("CADUCADA", "", None),
    );
    let countersigned = DocumentSignature {
        countersignatures: vec![DocumentSignature {
            countersignatures: vec![invalid_because(ValidityReason::Damaged)],
            ..a_signature("CONTRAFIRMA", "", None)
        }],
        ..a_signature("FIRMA", "", None)
    };

    let only_expired = in_json(vec![a_signature("VALIDA", "", None), expired.clone()]);
    let with_an_invalid_countersignature = in_json(vec![expired, countersigned]);

    assert_eq!(only_expired["validity"], "expired");
    assert_eq!(with_an_invalid_countersignature["validity"], "invalid");
    let nested = &with_an_invalid_countersignature["signatures"][1]["countersignatures"][0];
    assert_eq!(nested["signer"]["commonName"], "CONTRAFIRMA");
    assert_eq!(nested["validity"], "valid");
    assert_eq!(nested["countersignatures"][0]["reason"]["type"], "damaged");
}

#[test]
fn the_document_findings_go_at_the_root_by_their_name() {
    let verified = in_json_over(
        &["verify", "-i", "firmado.pdf", "-json"],
        &OneFile(A_PDF),
        &ReadingWithFindings(
            vec![a_signature("UNA PERSONA", "", None)],
            vec![
                DocumentFinding::ModifiedAfterLastSignature,
                DocumentFinding::FormFilledAfterSigning,
                DocumentFinding::ContentAddedOnTop,
            ],
        ),
    );

    assert_eq!(
        verified["findings"],
        json!([
            "modifiedAfterLastSignature",
            "formFilledAfterSigning",
            "contentAddedOnTop"
        ])
    );
}

#[test]
fn the_standard_is_the_one_of_the_signed_file() {
    let reader = Reading(Ok(vec![a_signature("UNA PERSONA", "", None)]));
    let standard_of = |file: &'static [u8]| {
        in_json_over(
            &["verify", "-i", "firmado", "-json"],
            &OneFile(file),
            &reader,
        )["standard"]
            .clone()
    };

    assert_eq!(standard_of(A_CMS), "CAdES");
    assert_eq!(standard_of(AN_XML), "XAdES");
    assert_eq!(standard_of(AN_INVOICE), "XAdES");
}

#[test]
fn verify_json_with_verbose_gives_the_same_json() {
    let reader = Reading(Ok(vec![a_complete_signature()]));
    let run = |words: &[&str]| {
        attended(
            words,
            &OneFile(A_PDF),
            &Answering::with(&[]),
            &reader,
            &Untouched,
        )
    };

    let plain = run(&["verify", "-i", "firmado.pdf", "-json"]);

    for verbose in ["-v", "-vv", "--verbose"] {
        assert_eq!(
            run(&["verify", verbose, "-i", "firmado.pdf", "--json"]),
            plain
        );
    }
}

#[test]
fn data_of_no_signature_format_give_an_empty_list_without_reaching_the_engine() {
    let verified = in_json_over(
        &["verify", "-i", "foto.png", "-json"],
        &OneFile(b"\x89PNG\r\n"),
        &Untouched,
    );

    assert_eq!(
        verified,
        json!({"standard": "unrecognized", "findings": [], "signatures": []})
    );
}

#[test]
fn a_pdf_without_signatures_gives_an_empty_list_and_no_validity() {
    let verified = in_json(Vec::new());

    assert_eq!(
        verified,
        json!({"standard": "PAdES", "findings": [], "signatures": []})
    );
}

#[test]
fn signatures_that_cannot_be_read_leave_stdout_empty_and_fail() {
    let outcome = attended(
        &["verify", "-i", "firmado.pdf", "-json"],
        &OneFile(A_PDF),
        &Answering::with(&["Firma valida"]),
        &Reading(Err("el isolate no arranca".to_owned())),
        &Untouched,
    );

    assert_ne!(outcome.exit_code, SUCCEEDED);
    assert!(outcome.stdout.is_empty(), "{:?}", outcome.stdout);
    assert!(
        outcome.stderr.join("\n").contains("el isolate no arranca"),
        "{:?}",
        outcome.stderr
    );
}

#[test]
fn verify_json_that_cannot_read_its_input_leaves_stdout_empty_and_fails() {
    let outcome = verified(
        &["verify", "-i", "no-esta.pdf", "-json"],
        &NoFile,
        &Answering::with(&[]),
    );

    assert_eq!(outcome.exit_code, FAILED);
    assert!(outcome.stdout.is_empty());
}
