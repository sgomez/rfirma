//! Clasificación de errores de material criptográfico TLS para la interfaz (ADR-0009).

use std::fmt;

/// Situación del fallo criptográfico TLS para su presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// No se pudo generar la CA local o el certificado del servidor local.
    MaterialNotGenerated,
    /// No se pudo guardar la CA local en disco.
    MaterialUnwritable,
    /// La CA local almacenada está dañada o corrupta.
    MaterialDamaged,
}

/// Error de material TLS con situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsError {
    situation: Situation,
    detail: String,
}

impl TlsError {
    /// Crea un nuevo fallo con su situación y detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// Situación clasificada del error.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// Detalle técnico del error.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TlsError {}

#[cfg(test)]
mod tests;
