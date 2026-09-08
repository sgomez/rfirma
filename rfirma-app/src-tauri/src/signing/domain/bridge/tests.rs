use super::{BridgeError, Format, PreSignBlock, PreSignature, TokenSignature, XadesVariant};
use crate::signing::domain::{SealMismatch, SessionSeal};

fn a_block(id: &str, pre: &[u8]) -> PreSignBlock {
    PreSignBlock {
        id: id.to_owned(),
        pre: pre.to_vec(),
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
            "XMLDSig Detached",
            "XMLDSig Enveloping",
            "XMLDSig Enveloped",
            "FacturaE",
        ]
    );
}

/// Los formatos que tienen pareja de entradas en el puente.
const BRIDGED: [Format; 7] = [
    Format::Pades,
    Format::Cades,
    Format::Cms,
    Format::Xades(XadesVariant::Detached),
    Format::Xades(XadesVariant::Enveloping),
    Format::Xades(XadesVariant::Enveloped),
    Format::Xades(XadesVariant::AsicS),
];

#[test]
fn the_bridge_resolves_pades_cades_cms_and_every_xades_variant_for_now() {
    for format in BRIDGED {
        assert_eq!(format.bridged().expect("tiene entradas"), format);
    }

    for format in Format::ALL
        .iter()
        .filter(|format| !BRIDGED.contains(format))
    {
        let refused = format.bridged().expect_err("no tiene entradas");

        assert!(matches!(refused, BridgeError::FormatNotBridged(named) if named == *format));
        assert!(refused.to_string().contains(format.name()));
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
