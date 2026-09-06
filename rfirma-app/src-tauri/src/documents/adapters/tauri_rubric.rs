//! Tipos de salida relacionados con la rúbrica y sus conversiones (ADR-0011, ADR-0012).

use std::io::Cursor;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::Serialize;

use crate::documents::adapters::rubric::NormalizedRubric;

pub use crate::commands::Failure;

/// Rúbrica normalizada con imagen en Base64 y dimensiones (ADR-0011, ADR-0012).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RubricView {
    /// Imagen JPEG normalizada en Base64.
    pub base64: String,
    /// Anchura en píxeles.
    pub width: u32,
    /// Altura en píxeles.
    pub height: u32,
}

impl RubricView {
    fn from_normalized(rubric: &NormalizedRubric) -> Self {
        Self::from_bytes(rubric.bytes())
    }

    /// Construye la vista a partir de los bytes JPEG del almacén.
    pub(super) fn from_bytes(jpeg: &[u8]) -> Self {
        let (width, height) =
            image::ImageReader::with_format(Cursor::new(jpeg), image::ImageFormat::Jpeg)
                .into_dimensions()
                .unwrap_or((0, 0));
        Self {
            base64: BASE64.encode(jpeg),
            width,
            height,
        }
    }
}

/// Resultado de la selección de una rúbrica.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RubricChoiceView {
    /// Rúbrica adoptada si la imagen era válida.
    pub rubric: Option<RubricView>,
    /// Causa del fallo si no se pudo adoptar la imagen.
    pub failure: Option<Failure>,
}

impl RubricChoiceView {
    /// Construye la respuesta de rúbrica adoptada con éxito.
    pub fn adopted(rubric: &NormalizedRubric) -> Self {
        Self {
            rubric: Some(RubricView::from_normalized(rubric)),
            failure: None,
        }
    }

    /// Construye la respuesta de rúbrica rechazada con el error correspondiente.
    pub fn refused(error: &crate::documents::adapters::rubric::RubricError) -> Self {
        Self {
            rubric: None,
            failure: Some(Failure::from(error)),
        }
    }
}

#[cfg(test)]
mod tests;
