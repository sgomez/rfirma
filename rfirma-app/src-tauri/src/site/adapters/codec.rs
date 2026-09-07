//! Códec del protocolo v4 para decodificar peticiones y codificar respuestas (ADR-0017).

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::site::domain::protocol::{
    read_operation, AfirmaUrl, Refusal, SafCode, SiteOperation, WireAnswer,
};

use crate::site::adapters::frontier;
use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};

const RESULT_SEPARATOR: char = '|';

/// El texto exacto que espera `autoscript.js` cuando el guardado sale bien (`CommandProcessorThread.java:295,336`).
const SAVE_OK: &str = "SAVE_OK";

/// Códec de la versión 4 del protocolo de comunicación con la sede.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct V4Codec;

impl ProtocolCodec for V4Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        match read_operation(message) {
            Ok(SiteOperation::SelectCertificate(request)) => {
                SiteRequest::SelectCertificate(request)
            }
            Ok(SiteOperation::Sign(request)) => SiteRequest::Sign(request),
            Ok(SiteOperation::Save(request)) => SiteRequest::Save(request),
            Ok(SiteOperation::Load(request)) => SiteRequest::Load(request),
            Ok(SiteOperation::SignAndSave(request)) => SiteRequest::SignAndSave(request),
            // El lote remoto se compone en el ticket del caso de uso (#481 solo
            // lo lee): hasta entonces, un lote leído sin componer es "no atendido".
            Ok(SiteOperation::Batch(_)) => SiteRequest::NotAttended(Refusal::new(
                SafCode::UnsupportedOperation,
                "el lote remoto todavia no se compone",
            )),
            Err(refusal) => SiteRequest::NotAttended(refusal),
        }
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        match outcome {
            SiteOutcome::Certificate(der) => on_the_wire(der),
            SiteOutcome::Signature { signer_der, signed } => {
                format!(
                    "{}{RESULT_SEPARATOR}{}",
                    on_the_wire(signer_der),
                    on_the_wire(signed)
                )
            }
            SiteOutcome::Saved => SAVE_OK.to_owned(),
            SiteOutcome::Loaded(files) => files
                .iter()
                .map(|(name, content)| format!("{name}:{}", STANDARD.encode(content)))
                .collect::<Vec<_>>()
                .join(&RESULT_SEPARATOR.to_string()),
            SiteOutcome::Cancelled => frontier::cancelled().on_the_wire(),
            SiteOutcome::Refused(refusal) => {
                WireAnswer::refused(frontier::code_of(refusal)).on_the_wire()
            }
            SiteOutcome::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
            SiteOutcome::Batch { result, signer_der } => match signer_der {
                Some(signer_der) => format!(
                    "{}{RESULT_SEPARATOR}{}",
                    STANDARD.encode(result),
                    STANDARD.encode(signer_der)
                ),
                None => STANDARD.encode(result),
            },
        }
    }
}

fn on_the_wire(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

#[cfg(test)]
mod tests;
