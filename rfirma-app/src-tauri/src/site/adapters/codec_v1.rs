//! Códec del protocolo v1 para decodificar peticiones y codificar respuestas (ADR-0017).

use crate::site::domain::protocol::AfirmaUrl;

use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};

use super::codec::{carries_extra_data, V4Codec};

/// Códec del transporte `service`: el mismo catálogo y la misma forma de respuesta que la
/// versión 4, medido contra el original, con el tercer componente según la versión negociada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct V1Codec {
    version: i64,
}

impl V1Codec {
    /// Un códec que habla la versión de `service` que la sede declaró.
    pub fn new(version: i64) -> Self {
        Self { version }
    }
}

impl ProtocolCodec for V1Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        V4Codec.decode(message)
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        match outcome {
            SiteOutcome::Signature { .. } if carries_extra_data(self.version) => {
                V4Codec.encode(outcome)
            }
            SiteOutcome::Signature {
                signer_der,
                signature,
                ..
            } => V4Codec.encode(&SiteOutcome::Signature {
                signer_der: signer_der.clone(),
                signature: signature.clone(),
                chosen_document: None,
            }),
            _ => V4Codec.encode(outcome),
        }
    }
}

#[cfg(test)]
mod tests;
