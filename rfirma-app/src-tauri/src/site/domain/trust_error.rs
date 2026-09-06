//! Clasificación de errores de almacenes NSS para la interfaz (ADR-0009).

use std::fmt;

/// Situación del fallo de almacén NSS para su presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Biblioteca NSS no disponible en el sistema.
    NssMissing,
    /// Almacén NSS inaccesible en lectura o escritura.
    StoreUnreachable,
    /// No se pudieron registrar los bits de confianza en el almacén.
    TrustNotWritten,
}

/// Error al interactuar con almacenes NSS con situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustError {
    situation: Situation,
    detail: String,
}

impl TrustError {
    /// Crea un nuevo fallo con situación y detalle técnico.
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

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for TrustError {}

#[cfg(test)]
mod tests;
