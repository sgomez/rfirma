//! **Lo que la ventana manda**: las órdenes tal y como llegan deserializadas.
//!
//! Van aparte de los tipos de salida de [`super::views`] porque entran en vez
//! de salir: aquí no hay ninguna guarda del ADR-0011 que valga —lo que llega no
//! puede filtrar una ruta del anfitrión, porque la ventana no tiene ninguna—.
//! Lo que sí hay es la única traducción que estos tipos hacen: la geometría del
//! visor a la del PDF.

use serde::Deserialize;

use crate::commands::Failure;
use crate::signing::{MediaBox, Page, Rotation, SignatureBox, UserSpaceRect};

/// Lo que la ventana ha marcado en las casillas del recuadro.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleFieldsOrder {
    pub signer_name: bool,
    pub id_number: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// Dónde ha caído el recuadro, tal como lo sabe el visor.
///
/// La `MediaBox` y la `/Rotate` las trae la ventana porque quien tiene abierto
/// el PDF es `pdf.js`: el backend **no lee PDFs**, y ponerle un analizador para
/// releer lo que el visor ya sabe sería una segunda opinión sobre la misma
/// página.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementOrder {
    /// Página **1-based**, como la numera `pdf.js` y como la cuenta
    /// `signaturePage`.
    pub page: u32,
    /// La `MediaBox` de esa página: `[x0, y0, x1, y1]`.
    pub media_box: [f64; 4],
    /// Su `/Rotate`, en grados.
    pub rotation: i32,
    /// El recuadro en espacio de usuario: `[x0, y0, x1, y1]`.
    ///
    /// En **coma flotante** porque eso es lo que sale de `convertToPdfPoint`
    /// del visor: pedir enteros aquí rechazaba la orden entera en el
    /// deserializador, antes de tocar el puente. El redondeo lo hace
    /// [`UserSpaceRect::rounded`], que es de quien es la regla.
    pub rect: [f64; 4],
}

impl PlacementOrder {
    /// El recuadro en puntos PAdES, o la negativa si se sale de la página.
    ///
    /// Es público porque quien lo llama es el caso de uso que arma la
    /// configuración de firma ([`crate::app::signing`]), y no una orden.
    pub fn signature_box(&self) -> Result<SignatureBox, Failure> {
        let [x0, y0, x1, y1] = self.media_box;
        let rotation = Rotation::from_degrees(self.rotation).ok_or_else(|| {
            Failure::new(
                "unknown",
                format!("una pagina no puede estar girada {} grados", self.rotation),
            )
        })?;
        let page = Page {
            number: self.page,
            media_box: MediaBox::new(x0, y0, x1, y1),
            rotation,
        };
        let [left, bottom, right, top] = self.rect;
        page.signature_box(&UserSpaceRect::rounded(left, bottom, right, top))
            .map_err(|out| Failure::new("boxOutOfPage", out.to_string()))
    }
}

/// La orden de firma completa: todo lo que distingue esta firma de otra.
///
/// `signed_at` llega **ya formateado** por la ventana, que es la que conoce el
/// huso y el formato de fecha del sistema, y es **el mismo** que se enseñó en
/// la vista previa: el recuadro se compone antes de la prefirma y el PDF ya no
/// se vuelve a tocar, así que enseñar una hora y estampar otra sería enseñar
/// algo que el PDF no va a tener.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningOrder {
    /// El asa que dio el portal al abrir el documento. Entra, no sale.
    pub document: String,
    /// El **asa** del certificado elegido, la que dio
    /// [`CertificateView::id`](super::views::CertificateView::id). Entra, no
    /// sale, y no es la etiqueta: con dos etiquetas iguales la etiqueta no
    /// distingue una fila de la otra.
    pub certificate: String,
    /// Dónde cae el recuadro, en **espacio de usuario PDF** (ID-21).
    ///
    /// No en puntos PAdES: la inversa de la `/Rotate` que iText aplica al
    /// cerrar el documento la hace [`crate::signing::placement`], y con ella
    /// viene gratis la guardia del ID-22 —un recuadro que se saliera de la
    /// página iText **lo recorta en silencio** y la firma sale válida igual—.
    pub placement: PlacementOrder,
    pub fields: VisibleFieldsOrder,
    /// El motivo. Vacío es «sin motivo».
    pub reason: String,
    /// La fecha y hora, ya formateadas.
    pub signed_at: String,
    /// La rúbrica en JPEG y Base64, ya normalizada por [`crate::rubric`].
    pub rubric: Option<String>,
    /// El idioma en el que se componen las etiquetas del recuadro.
    pub language: String,
}

#[cfg(test)]
mod tests {
    use super::PlacementOrder;

    /// La ventana manda el recuadro en espacio de usuario **tal cual sale de
    /// `convertToPdfPoint`**, y eso son fracciones: `pdf.js` invierte una
    /// matriz, no cuenta puntos enteros.
    #[test]
    fn accepts_a_rect_with_the_fractional_coordinates_the_viewer_sends() {
        let sent = serde_json::json!({
            "page": 1,
            "mediaBox": [0.0, 0.0, 595.276, 841.89],
            "rotation": 0,
            "rect": [47.7218, 179.1376722440945, 250.1, 259.9],
        });

        let placement: PlacementOrder =
            serde_json::from_value(sent).expect("el recuadro del visor tiene decimales");

        let box_ = placement.signature_box().expect("cabe en la pagina");
        assert_eq!(
            (
                box_.lower_left_x,
                box_.lower_left_y,
                box_.upper_right_x,
                box_.upper_right_y
            ),
            (48, 179, 250, 260)
        );
    }
}
