use base64::Engine;

use super::BarePkcs1;
use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignRequest, SignatureOperation,
};
use crate::signing::ports::Bridge;

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn a_presign_of(operation: SignatureOperation, document_b64: &str) -> PreSignRequest<'_> {
    PreSignRequest {
        format: Format::Pkcs1,
        operation,
        document_b64,
        algorithm: "SHA256withRSA",
        certificate_chain_b64: "",
        extra_params: "",
    }
}

#[test]
fn the_only_block_the_token_signs_is_the_data_itself() {
    let data = b64(b"los datos de la sede");

    let presigned = BarePkcs1
        .presign(a_presign_of(SignatureOperation::Sign, &data))
        .expect("una firma NONE se prefirma sin puente");

    let blocks: Vec<&[u8]> = presigned
        .blocks()
        .iter()
        .map(|block| block.pre_sign())
        .collect();
    assert_eq!(blocks, [b"los datos de la sede".as_slice()]);
}

#[test]
fn what_comes_out_is_the_pkcs1_of_the_token_with_nothing_around() {
    let data = b64(b"los datos de la sede");
    let presigned = BarePkcs1
        .presign(a_presign_of(SignatureOperation::Sign, &data))
        .expect("prefirma");
    let signatures = presigned
        .signed_one_by_one(|_| Ok::<_, ()>(b"el PKCS#1 del token".to_vec()))
        .expect("el token firma");
    let sealed = presigned
        .sealed_with(signatures, presigned.stamp())
        .expect("el sello vuelve intacto");

    let signed = BarePkcs1
        .postsign(PostSignRequest {
            format: Format::Pkcs1,
            document_b64: &data,
            certificate_chain_b64: "",
            sealed: &sealed,
        })
        .expect("una firma NONE se postfirma sin puente");

    assert_eq!(signed, b"el PKCS#1 del token");
}

#[test]
fn a_bare_pkcs1_is_neither_cosigned_nor_countersigned() {
    let data = b64(b"una firma NONE anterior");

    for operation in [SignatureOperation::Cosign, SignatureOperation::Countersign] {
        let refused = BarePkcs1
            .presign(a_presign_of(operation, &data))
            .expect_err("AOPkcs1Signer no multifirma");

        assert!(
            matches!(
                refused,
                BridgeError::UnsupportedOperation(Format::Pkcs1, asked) if asked == operation
            ),
            "{operation:?}: {refused}"
        );
    }
}

#[test]
fn data_that_is_not_base64_is_refused_before_the_token() {
    let refused = BarePkcs1
        .presign(a_presign_of(SignatureOperation::Sign, "esto no es base64"))
        .expect_err("no hay datos que firmar");

    assert!(
        matches!(refused, BridgeError::InvalidArgument(_)),
        "{refused}"
    );
}
