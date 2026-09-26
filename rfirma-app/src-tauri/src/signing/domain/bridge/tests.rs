use super::{
    BridgeError, Format, PreSignBlock, PreSignature, PreviousSignature, PreviousSignaturesReport,
    SignatureStatus, TokenSignature, Tone, XadesVariant,
};
use crate::signing::domain::{SealMismatch, SessionSeal};

fn a_block(id: &str, pre: &[u8]) -> PreSignBlock {
    PreSignBlock {
        id: id.to_owned(),
        pre: pre.to_vec(),
    }
}

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

fn a_presignature() -> PreSignature {
    PreSignature {
        session: "<xml/>".to_owned(),
        blocks: vec![a_block("uno", b"123")],
        stamp: SessionSeal::from_bridge("el sello de la prefirma"),
    }
}

#[test]
fn the_signature_travels_to_the_postsign_in_base64() {
    let signature = TokenSignature::from_token(vec![0x30, 0x82, 0x01, 0x00]);

    assert_eq!(signature.raw(), [0x30, 0x82, 0x01, 0x00]);
    assert_eq!(signature.to_pkcs1_base64(), "MIIBAA==");
}

#[test]
fn a_seal_that_came_back_intact_seals_the_presignature_with_the_signature() {
    let presigned = a_presignature();

    let sealed = presigned
        .sealed_with(presigned.invented_signatures(), presigned.stamp())
        .expect("el sello es el mismo");

    assert_eq!(sealed.session(), "<xml/>");
    assert_eq!(sealed.stamp(), presigned.stamp());
    assert_eq!(sealed.signed().len(), 1);
    assert_eq!(sealed.signed()[0].id(), "uno");
    assert_eq!(
        sealed.signed()[0].pkcs1_b64(),
        TokenSignature::invented().to_pkcs1_base64()
    );
}

#[test]
fn a_seal_that_came_back_changed_is_refused_before_anything_else() {
    let presigned = a_presignature();
    let tampered = SessionSeal::from_bridge("el sello de la prefirma.");

    let refused = presigned.sealed_with(presigned.invented_signatures(), &tampered);

    assert_eq!(refused.unwrap_err(), SealMismatch);
}

#[test]
fn a_completed_cycle_carries_the_pdf_the_postsign_returned() {
    let presigned = a_presignature();
    let sealed = presigned
        .sealed_with(presigned.invented_signatures(), presigned.stamp())
        .expect("el sello es el mismo");

    let completed = sealed.completed_with(b"%PDF-".to_vec());

    assert_eq!(completed.signed_document(), b"%PDF-");
    assert_eq!(completed.into_signed_document(), b"%PDF-".to_vec());
}

#[test]
fn every_format_says_the_name_the_original_expects() {
    let names: Vec<&str> = Format::ALL.iter().map(|format| format.name()).collect();

    assert_eq!(
        names,
        [
            "PAdES",
            "CAdES",
            "CAdES-ASiC-S",
            "CMS/PKCS#7",
            "XAdES Detached",
            "XAdES Enveloping",
            "XAdES Enveloped",
            "XAdES-ASiC-S",
            "FacturaE",
            "NONE",
        ]
    );
}

#[test]
fn the_bridge_resolves_every_format_of_the_vocabulary_but_the_bare_pkcs1() {
    for format in Format::ALL {
        if format == Format::Pkcs1 {
            continue;
        }
        assert_eq!(format.bridged().expect("tiene entradas"), format);
        assert!(!format.signed_without_the_bridge(), "{format}");
    }
}

#[test]
fn a_bare_pkcs1_never_crosses_to_the_bridge() {
    assert!(Format::Pkcs1.signed_without_the_bridge());
    assert!(matches!(
        Format::Pkcs1.bridged(),
        Err(BridgeError::FormatNotBridged(Format::Pkcs1))
    ));
}

#[test]
fn the_containers_and_the_bare_pkcs1_have_no_validator_of_their_own_in_the_original() {
    for format in [
        Format::CadesAsicS,
        Format::Xades(XadesVariant::AsicS),
        Format::Pkcs1,
    ] {
        assert!(
            matches!(
                format.validated(),
                Err(BridgeError::FormatNotBridged(refused)) if refused == format
            ),
            "{format} no puede llegar al validador"
        );
    }

    for format in Format::ALL {
        if matches!(
            format,
            Format::CadesAsicS | Format::Xades(XadesVariant::AsicS) | Format::Pkcs1
        ) {
            continue;
        }
        assert_eq!(format.validated().expect("tiene validador"), format);
    }
}

#[test]
fn every_block_of_a_countersignature_is_signed_with_the_same_secret() {
    let presigned = PreSignature {
        session: "<xml/>".to_owned(),
        blocks: vec![
            a_block("uno", b"1"),
            a_block("dos", b"2"),
            a_block("tres", b"3"),
        ],
        stamp: SessionSeal::from_bridge("el sello de la prefirma"),
    };
    let mut asked = Vec::new();

    let signatures = presigned
        .signed_one_by_one(|pre| {
            asked.push(pre.to_vec());
            Ok::<_, SealMismatch>(pre.to_vec())
        })
        .expect("el token firma cada bloque");
    let sealed = presigned
        .sealed_with(signatures, presigned.stamp())
        .expect("el sello es el mismo");

    assert_eq!(asked, [b"1".to_vec(), b"2".to_vec(), b"3".to_vec()]);
    let identifiers: Vec<&str> = sealed.signed().iter().map(|block| block.id()).collect();
    assert_eq!(identifiers, ["uno", "dos", "tres"]);
}

#[test]
fn a_signature_operation_says_the_name_the_bridge_expects() {
    let names: Vec<&str> = [
        super::SignatureOperation::Sign,
        super::SignatureOperation::Cosign,
        super::SignatureOperation::Countersign,
    ]
    .iter()
    .map(|operation| operation.name())
    .collect();

    assert_eq!(names, ["sign", "cosign", "countersign"]);
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
