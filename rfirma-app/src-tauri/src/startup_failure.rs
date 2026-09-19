//! Clasificación de los fallos de arranque de los dos roles, sin instancia de webview donde pintarlos.

use std::fmt;

/// Dirección del repositorio, mostrada como texto seleccionable en el diálogo de arranque.
pub const REPOSITORY_ADDRESS: &str = "github.com/sgomez/rfirma";

/// Situación de fallo al arrancar, clasificable en un mensaje fijo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// No se ha podido determinar el directorio personal de la persona usuaria.
    HomeUnknown,
    /// No se ha podido crear la carpeta de paso de este proceso.
    ScratchFolderUnusable,
    /// No se ha podido levantar la ventana de la aplicación.
    WindowUnavailable,
}

/// Fallo de arranque con su situación y el detalle técnico crudo, sin traducir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupFailure {
    situation: Situation,
    detail: String,
}

impl StartupFailure {
    /// Construye un fallo de arranque a partir de su situación y el detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// La situación clasificada.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// El detalle técnico crudo. Nunca traducido.
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// La frase en castellano de la situación, la que ve la persona usuaria.
    pub fn phrase(&self) -> &'static str {
        phrase_of(self.situation)
    }
}

impl fmt::Display for StartupFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.phrase(), self.detail)
    }
}

impl std::error::Error for StartupFailure {}

fn phrase_of(situation: Situation) -> &'static str {
    match situation {
        Situation::HomeUnknown => "rFirma no sabe cuál es tu carpeta personal y no puede arrancar.",
        Situation::ScratchFolderUnusable => {
            "rFirma no ha podido crear la carpeta de paso que necesita y no puede arrancar."
        }
        Situation::WindowUnavailable => "rFirma no ha podido abrir la ventana y no puede arrancar.",
    }
}

#[cfg(test)]
mod tests;
