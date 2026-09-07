//! Situaciones del lote remoto: alcance de los dos servlets y forma de su respuesta (ADR-0009).

use std::fmt;

/// Situación del fallo del lote remoto para su presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El servlet de prefirma no respondió.
    PresignerUnreachable,
    /// El servlet de postfirma no respondió.
    PostsignerUnreachable,
    /// La respuesta de prefirma no tiene la forma esperada.
    InvalidPresignResponse,
    /// La respuesta de postfirma no tiene la forma esperada.
    InvalidPostsignResponse,
}

/// Error del lote remoto con situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchError {
    situation: Situation,
    detail: String,
}

impl BatchError {
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

impl fmt::Display for BatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for BatchError {}

#[cfg(test)]
mod tests;
