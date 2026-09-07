//! Códec del servidor intermedio: cifra cada campo de la respuesta con la clave negociada (ADR-0017).

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::site::domain::protocol::{
    encrypt, read_operation, AfirmaUrl, CipherKey, SiteOperation, WireAnswer,
};

use crate::site::adapters::frontier;
use crate::site::application::errand::{ProtocolCodec, SiteOutcome, SiteRequest};

const RESULT_SEPARATOR: char = '|';

/// Códec del servidor intermedio: lee las operaciones igual que la versión 4, pero cifra cada
/// campo de la respuesta con la clave negociada, calcado de `NativeSignDataProcessor` (1.9.2).
#[derive(Clone, Debug)]
pub struct RelayCodec {
    key: Option<CipherKey>,
}

impl RelayCodec {
    /// Un códec con la clave que la sede negoció para este servidor intermedio, si la hay.
    pub fn new(key: Option<CipherKey>) -> Self {
        Self { key }
    }

    fn on_the_wire(&self, bytes: &[u8]) -> String {
        match &self.key {
            Some(key) => encrypt(bytes, key),
            None => STANDARD.encode(bytes),
        }
    }
}

impl ProtocolCodec for RelayCodec {
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
            SiteOutcome::Certificate(der) => self.on_the_wire(der),
            SiteOutcome::Signature { signer_der, signed } => {
                format!(
                    "{}{RESULT_SEPARATOR}{}",
                    self.on_the_wire(signer_der),
                    self.on_the_wire(signed)
                )
            }
            // El error sube en claro y sin cifrar, como el original (ProtocolInvocationLauncher.java).
            SiteOutcome::Cancelled => frontier::cancelled().on_the_wire(),
            SiteOutcome::Refused(refusal) => {
                WireAnswer::refused(frontier::code_of(refusal)).on_the_wire()
            }
            SiteOutcome::RefusedByTheProtocol(refusal) => refusal.answer().on_the_wire(),
        }
    }
}

#[cfg(test)]
mod tests;
