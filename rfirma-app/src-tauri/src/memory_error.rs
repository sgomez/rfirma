//! Clasificación de errores de persistencia entre sesiones (ADR-0009, ADR-0010).

use std::fmt;
use std::path::Path;

/// Situación de fallo en persistencia clasificable para la interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El fichero existe pero no se ha podido leer.
    Unreadable,
    /// No se ha podido escribir en el soporte de persistencia.
    Unwritable,
}

/// Error de persistencia con situación y detalle técnico (ADR-0009).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryError {
    situation: Situation,
    detail: String,
}

impl MemoryError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// El fallo de un fichero concreto, con la ruta dentro del detalle.
    pub fn about(situation: Situation, path: &Path, error: &std::io::Error) -> Self {
        Self::new(situation, format!("{}: {error}", path.display()))
    }

    /// La situación que la interfaz enseña, ya clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El detalle técnico crudo. Nunca vacío, nunca traducido.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for MemoryError {}

#[cfg(test)]
mod tests;
