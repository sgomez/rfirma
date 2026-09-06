//! Códec del protocolo v4 para decodificar peticiones y codificar respuestas (ADR-0017).

use base64::Engine as _;

use crate::site::domain::protocol::{read_operation, AfirmaUrl, SiteOperation};

use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};
use crate::site::application::frontier;

const RESULT_SEPARATOR: char = '|';

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
            SiteOutcome::Cancelled => frontier::cancelled().on_the_wire(),
            SiteOutcome::Refused { answer, .. } => answer.on_the_wire(),
            SiteOutcome::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
        }
    }
}

fn on_the_wire(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE.encode(bytes)
}

#[cfg(test)]
mod tests;
