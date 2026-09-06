//! Clasificación de situaciones de fallo al procesar la rúbrica (ADR-0009, ADR-0012).

use std::fmt;

/// Situaciones de fallo de la rúbrica traducibles por el catálogo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// El fichero no es un formato admitido (PNG o JPEG).
    NotAnAcceptedImage,
    /// La imagen está dañada o no se puede decodificar.
    DamagedImage,
    /// El fichero excede el tamaño máximo permitido.
    ImageTooLarge,
    /// No se ha podido leer el fichero de origen.
    SourceUnreadable,
    /// No se ha podido escribir en el almacén de rúbricas.
    StoreUnwritable,
    /// No se ha podido leer la rúbrica del almacén.
    StoreUnreadable,
}

/// Fallo de la rúbrica clasificado con situación y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubricError {
    situation: Situation,
    detail: String,
}

impl RubricError {
    /// Construye un fallo con su situación y detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// Situación clasificada para la interfaz.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// Detalle técnico del fallo.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for RubricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for RubricError {}

#[cfg(test)]
mod tests;
