//! **Lo que cruza a la ventana sobre la rúbrica**: separado de [`super::views`]
//! por tamaño, no porque sea una cosa distinta (ID-80) — el mismo motivo por
//! el que [`super::failure`] va aparte.

use std::io::Cursor;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::Serialize;

use crate::rubric::NormalizedRubric;

pub use super::failure::Failure;

/// La rúbrica ya normalizada, tal como la ventana la enseña: el JPEG en
/// Base64 y sus dimensiones, sin el prefijo `data:` —lo antepone la
/// ventana, que es quien sabe que es para un `<img>`.
///
/// No lleva ninguna ruta: es el JPEG que
/// [`crate::rubric::RubricStore::adopt`] acaba de copiar al almacén de la
/// aplicación, no el fichero que el usuario eligió (ID-82, ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RubricView {
    /// El JPEG normalizado, en Base64.
    pub base64: String,
    pub width: u32,
    pub height: u32,
}

impl RubricView {
    /// Lee las dimensiones de la cabecera del JPEG sin decodificarlo entero:
    /// `rubric::normalize` no las guarda —no las necesita para nada— así que
    /// se preguntan aquí, y no tocando ese módulo (ID-82 no cambia lo que
    /// `normalize` hace con la imagen).
    ///
    /// Una cabecera ilegible no debería darse nunca —el JPEG lo acaba de
    /// producir `normalize`, o lo dejó escrito una sesión anterior que ya lo
    /// hizo—, pero es la única entrada de este módulo que viene de fuera de
    /// la orden que la produjo: si algún día lo está, `(0, 0)` no revienta el
    /// proceso por un dato que solo afecta a cómo se pinta el recuadro.
    fn from_normalized(rubric: &NormalizedRubric) -> Self {
        Self::from_bytes(rubric.bytes())
    }

    /// Lo mismo que [`Self::from_normalized`], pero partiendo de los bytes
    /// crudos del almacén en vez de un [`NormalizedRubric`] recién producido:
    /// es lo que lee `read_rubric` al arrancar, cuando la rúbrica no la
    /// acaba de adoptar esta sesión.
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

/// Lo que devuelve elegir una rúbrica: la imagen ya adoptada, o por qué no se
/// ha podido. Las dos caras no se funden en un solo `Result` porque cruzan a
/// TypeScript campo a campo, igual que `DroppedDocumentView` hace con «se
/// abrió o no se abrió» (ID-80).
///
/// Cancelar el diálogo **no** es esto: es el `None` de fuera,
/// `Option<RubricChoiceView>`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RubricChoiceView {
    /// La rúbrica adoptada, si la imagen elegida era válida.
    pub rubric: Option<RubricView>,
    /// Por qué no se ha adoptado. `None` cuando sí se adoptó.
    pub failure: Option<Failure>,
}

impl RubricChoiceView {
    /// La rúbrica se ha adoptado.
    pub fn adopted(rubric: &NormalizedRubric) -> Self {
        Self {
            rubric: Some(RubricView::from_normalized(rubric)),
            failure: None,
        }
    }

    /// No se ha podido adoptar: la situación clasificada y el detalle crudo,
    /// con los mismos nombres que `RubricSituation` en TypeScript.
    pub fn refused(error: &crate::rubric::RubricError) -> Self {
        Self {
            rubric: None,
            failure: Some(Failure::from(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RubricChoiceView;
    use crate::rubric::{normalize, RubricError, Situation};

    /// **Grada A**: son bytes en memoria, no hace falta ni token ni puente.
    fn a_normalized_rubric() -> crate::rubric::NormalizedRubric {
        let mut png = Vec::new();
        image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 2, 3, 255]))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("el PNG de prueba deberia codificarse");
        normalize(&png).expect("el PNG de prueba deberia normalizarse")
    }

    #[test]
    fn an_adopted_rubric_crosses_with_its_base64_and_size_and_no_failure() {
        let rubric = a_normalized_rubric();

        let choice = RubricChoiceView::adopted(&rubric);

        assert_eq!(
            choice.rubric.as_ref().map(|view| &view.base64),
            Some(&rubric.to_base64())
        );
        assert_eq!(
            choice.rubric.as_ref().map(|view| (view.width, view.height)),
            Some((4, 4))
        );
        assert!(choice.failure.is_none());
    }

    #[test]
    fn a_refused_rubric_crosses_with_its_situation_and_no_image() {
        let error = RubricError::new(Situation::NotAnAcceptedImage, "no es PNG ni JPEG");

        let choice = RubricChoiceView::refused(&error);

        assert!(choice.rubric.is_none());
        assert_eq!(
            choice
                .failure
                .as_ref()
                .map(|failure| failure.situation.as_str()),
            Some("notAnAcceptedImage")
        );
        assert_eq!(
            choice
                .failure
                .as_ref()
                .map(|failure| failure.detail.as_str()),
            Some("no es PNG ni JPEG")
        );
    }
}
