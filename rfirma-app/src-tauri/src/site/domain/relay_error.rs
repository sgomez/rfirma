//! Situaciones del servidor intermedio: cifrado, alcance del servlet y subida (ADR-0009).

use std::fmt;

/// Situación del fallo del servidor intermedio para su presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El descifrado o el cifrado del original no se pudo completar.
    DecryptionFailed,
    /// El servlet de almacenamiento o recuperación no respondió.
    ServletUnreachable,
    /// El servlet rechazó la subida de datos.
    UploadRejected,
}

/// Error del servidor intermedio con situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayError {
    situation: Situation,
    detail: String,
}

impl RelayError {
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

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for RelayError {}

#[cfg(test)]
mod tests;
