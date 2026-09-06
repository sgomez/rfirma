use super::{PreSignature, TokenSignature};
use crate::signing::domain::{SealMismatch, SessionSeal};

fn a_presignature() -> PreSignature {
    PreSignature {
        session: "<xml/>".to_owned(),
        pre_sign: b"123".to_vec(),
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
        .sealed_with(&TokenSignature::invented(), presigned.stamp())
        .expect("el sello es el mismo");

    assert_eq!(sealed.session(), "<xml/>");
    assert_eq!(sealed.stamp(), presigned.stamp());
    assert_eq!(
        sealed.pkcs1_b64(),
        TokenSignature::invented().to_pkcs1_base64()
    );
}

#[test]
fn a_seal_that_came_back_changed_is_refused_before_anything_else() {
    let presigned = a_presignature();
    let tampered = SessionSeal::from_bridge("el sello de la prefirma.");

    let refused = presigned.sealed_with(&TokenSignature::invented(), &tampered);

    assert_eq!(refused.unwrap_err(), SealMismatch);
}

#[test]
fn a_completed_cycle_carries_the_pdf_the_postsign_returned() {
    let presigned = a_presignature();
    let sealed = presigned
        .sealed_with(&TokenSignature::invented(), presigned.stamp())
        .expect("el sello es el mismo");

    let completed = sealed.completed_with(b"%PDF-".to_vec());

    assert_eq!(completed.pdf(), b"%PDF-");
    assert_eq!(completed.into_pdf(), b"%PDF-".to_vec());
}
