//! Cómo se le cuenta a la ventana que algo salió mal (ADR-0009); cada contexto traduce lo suyo en su `adapters/failures.rs`.

use serde::Serialize;

/// Representación de un fallo devuelto a la ventana (ADR-0009).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    /// Identificador en camelCase de la situación.
    pub situation: String,
    /// Detalle descriptivo original del error.
    pub detail: String,
    /// Intentos restantes cuando aplique.
    pub attempts_left: Option<u32>,
}

impl Failure {
    /// Construye un nuevo fallo a partir de su situación y detalle.
    pub fn new(situation: &str, detail: impl Into<String>) -> Self {
        Self {
            situation: situation.to_owned(),
            detail: detail.into(),
            attempts_left: None,
        }
    }
}

#[cfg(test)]
mod tests;
