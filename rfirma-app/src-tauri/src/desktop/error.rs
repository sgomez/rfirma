//! Los fallos de elegir manejador **se clasifican, no se traducen**
//! (ADR-0009), igual que los del canal en [`crate::channel::error`].
//!
//! Son tres situaciones y no más, porque desde fuera solo hay tres remedios
//! distintos: dentro del sandbox no hay nada que hacer, un `mimeapps.list`
//! ilegible no se pisa, y uno que no se deja escribir es cosa de permisos.

use std::fmt;

/// Situación al leer o escribir quién atiende un esquema (ADR-0009).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Dentro del flatpak no hay `mimeapps.list` del anfitrión que valga
    /// (ID-240): ni se lee ni se escribe, y no se finge que sí.
    NotAvailableInsideTheSandbox,
    /// El `mimeapps.list` de la persona existe y no se ha podido leer. No se
    /// escribe encima: reescribirlo entero perdería lo que hubiera dentro.
    TheListIsNotReadable,
    /// El `mimeapps.list` de la persona no se ha podido escribir.
    TheListIsNotWritable,
}

/// Un fallo del registro del esquema: la situación traducible y el detalle
/// crudo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopError {
    situation: Situation,
    detail: String,
}

impl DesktopError {
    /// Un fallo con su detalle técnico, sin traducir.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
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

impl fmt::Display for DesktopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for DesktopError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// El fallo lleva su detalle crudo al lado de la situación (ADR-0009).
    #[test]
    fn a_failure_keeps_its_untranslated_detail_next_to_the_situation() {
        let error = DesktopError::new(Situation::TheListIsNotWritable, "Permission denied");

        assert_eq!(error.situation(), Situation::TheListIsNotWritable);
        assert_eq!(error.detail(), "Permission denied");
        assert!(error.to_string().contains("TheListIsNotWritable"));
        assert!(error.to_string().contains("Permission denied"));
    }
}
