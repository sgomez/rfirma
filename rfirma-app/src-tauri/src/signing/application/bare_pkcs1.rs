//! La firma NONE hecha en Rust: el token firma los datos tal cual y sale su PKCS#1, sin puente (ADR-0001).

use base64::Engine;

use crate::signing::domain::bridge::{
    BridgeError, Format, PostSignRequest, PreSignBlock, PreSignRequest, PreSignature,
    SignatureOperation,
};
use crate::signing::domain::SessionSeal;
use crate::signing::ports::Bridge;

const THE_DATA: &str = "NONE";

/// El `Bridge` de la firma NONE, que no cruza la frontera.
pub struct BarePkcs1;

impl Bridge for BarePkcs1 {
    fn presign(&self, request: PreSignRequest<'_>) -> Result<PreSignature, BridgeError> {
        if request.operation != SignatureOperation::Sign {
            return Err(BridgeError::UnsupportedOperation(
                Format::Pkcs1,
                request.operation,
            ));
        }
        Ok(PreSignature {
            session: String::new(),
            blocks: vec![PreSignBlock {
                id: THE_DATA.to_owned(),
                pre: decoded(request.document_b64, "los datos")?,
            }],
            stamp: SessionSeal::from_bridge(""),
        })
    }

    fn postsign(&self, request: PostSignRequest<'_>) -> Result<Vec<u8>, BridgeError> {
        match request.sealed.signed() {
            [only] => decoded(only.pkcs1_b64(), "el PKCS#1"),
            blocks => Err(BridgeError::MalformedResponse(format!(
                "una firma NONE lleva un PKCS#1, y llegaron {}",
                blocks.len()
            ))),
        }
    }
}

fn decoded(b64: &str, what: &'static str) -> Result<Vec<u8>, BridgeError> {
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| BridgeError::InvalidArgument(what))
}

#[cfg(test)]
mod tests;
