//! Códec del protocolo v3 para decodificar peticiones y codificar respuestas (ADR-0017).

use crate::site::domain::protocol::AfirmaUrl;

use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};

use super::codec::V4Codec;

/// Códec de la versión 3 del protocolo de comunicación con la sede: el mismo catálogo y la
/// misma forma de respuesta que la versión 4, medido contra el original.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct V3Codec;

impl ProtocolCodec for V3Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        V4Codec.decode(message)
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        V4Codec.encode(outcome)
    }
}

#[cfg(test)]
mod tests;
