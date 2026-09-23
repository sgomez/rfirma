//! Códec del protocolo v3 para decodificar peticiones y codificar respuestas (ADR-0017).

use crate::site::domain::protocol::{read_operation_within_the_protocol, AfirmaUrl};

use crate::site::adapters::data_download::HttpDataSource;
use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};

use super::codec::{request_of, V4Codec};

/// Códec de la versión 3 del protocolo de comunicación con la sede: el catálogo y la forma
/// de respuesta de la versión 4, pero cada operación se somete a la versión que declara.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct V3Codec;

impl ProtocolCodec for V3Codec {
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest {
        request_of(read_operation_within_the_protocol(message, &HttpDataSource))
    }

    fn encode(&self, outcome: &SiteOutcome) -> String {
        V4Codec.encode(outcome)
    }
}

#[cfg(test)]
mod tests;
