//! Clasificación de errores al consultar o registrar manejadores del escritorio (ADR-0009).

use std::fmt;

/// Situación al leer o escribir quién atiende un esquema (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// No disponible dentro del sandbox flatpak.
    NotAvailableInsideTheSandbox,
    /// El fichero mimeapps.list no se ha podido leer.
    TheListIsNotReadable,
    /// El fichero mimeapps.list no se ha podido escribir.
    TheListIsNotWritable,
}

/// Fallo del registro de manejadores con situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopError {
    situation: Situation,
    detail: String,
}

impl DesktopError {
    /// Construye un fallo con su situación y detalle técnico.
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

    /// Detalle técnico sin traducir.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for DesktopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for DesktopError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = DesktopError::new(Situation::TheListIsNotWritable, "Permission denied");

        assert_eq!(error.situation(), Situation::TheListIsNotWritable);
        assert_eq!(error.detail(), "Permission denied");
        assert!(error.to_string().contains("TheListIsNotWritable"));
        assert!(error.to_string().contains("Permission denied"));
    }
}
