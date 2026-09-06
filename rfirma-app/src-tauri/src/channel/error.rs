//! Clasificación de errores del canal local para la interfaz (ADR-0009).

use std::fmt;

/// Situación del fallo del canal para su presentación en interfaz (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Ninguno de los puertos sorteados por la sede estaba libre.
    NoDrawnPortIsFree,
    /// El material criptográfico no puede utilizarse para la conexión TLS.
    MaterialNotUsable,
    /// Error del sistema al intentar iniciar la escucha en el socket.
    NotListening,
}

/// Fallo del canal compuesto por situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelError {
    situation: Situation,
    detail: String,
}

impl ChannelError {
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

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for ChannelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = ChannelError::new(Situation::NoDrawnPortIsFree, "Address already in use");

        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
        assert_eq!(error.detail(), "Address already in use");
        assert!(error.to_string().contains("NoDrawnPortIsFree"));
        assert!(error.to_string().contains("Address already in use"));
    }
}
