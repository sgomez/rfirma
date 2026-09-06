//! Deserialización y validación de las órdenes de la ventana.

use serde::Deserialize;

use crate::commands::Failure;
use crate::signing::{MediaBox, Page, PageSet, Placement, Rotation, UserSpaceRect};

/// Lo que la ventana ha marcado en las casillas del recuadro.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VisibleFieldsOrder {
    pub signer_name: bool,
    pub issuer: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// Dónde ha caído el recuadro, tal como lo sabe el visor.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementOrder {
    /// La página sobre la que se arrastró, 1-based como la numera `pdf.js`.
    pub page: u32,
    /// En qué páginas se estampa.
    pub pages: PageSet,
    /// Cuántas páginas tiene el documento, según el visor.
    pub page_count: u32,
    /// La `MediaBox` de la página del arrastre: `[x0, y0, x1, y1]`.
    pub media_box: [f64; 4],
    /// Su `/Rotate`, en grados.
    pub rotation: i32,
    /// El recuadro en espacio de usuario: `[x0, y0, x1, y1]`.
    pub rect: [f64; 4],
}

impl PlacementOrder {
    /// La colocación en puntos PAdES, o la negativa si el destino no existe o el recuadro se sale de la página.
    pub fn placement(&self) -> Result<Placement, Failure> {
        self.pages
            .validate(self.page_count)
            .map_err(|out| Failure::new(PAGE_OUT_OF_DOCUMENT, out.to_string()))?;
        PageSet::only_page(self.page)
            .validate(self.page_count)
            .map_err(|out| Failure::new(PAGE_OUT_OF_DOCUMENT, out.to_string()))?;

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
        let rect = page
            .pades_rect(&UserSpaceRect::rounded(left, bottom, right, top))
            .map_err(|out| Failure::new("boxOutOfPage", out.to_string()))?;
        Ok(Placement {
            rect,
            pages: self.pages.clone(),
        })
    }
}

/// La situación con la que la ventana pinta un destino que el documento no tiene.
const PAGE_OUT_OF_DOCUMENT: &str = "pageOutOfDocument";

/// La orden de firma completa: todo lo que distingue esta firma de otra.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningOrder {
    /// El asa que dio el portal al abrir el documento.
    pub document: String,
    /// El asa del certificado elegido.
    pub certificate: String,
    /// Dónde cae el recuadro, en espacio de usuario PDF.
    pub placement: Option<PlacementOrder>,
    pub fields: VisibleFieldsOrder,
    /// El motivo, o vacío si no se especifica.
    pub reason: String,
    /// La fecha y hora, ya formateadas.
    pub signed_at: String,
    /// La rúbrica en JPEG y Base64, ya normalizada.
    pub rubric: Option<String>,
    /// El idioma en el que se componen las etiquetas del recuadro.
    pub language: String,
    /// Si la persona ha consentido cofirmar un PDF con firmas no reconocidas.
    #[serde(default)]
    pub allow_unregistered_signatures: bool,
}

#[cfg(test)]
mod tests {
    use super::PlacementOrder;
    use crate::signing::PageSet;

    #[test]
    fn accepts_a_rect_with_the_fractional_coordinates_the_viewer_sends() {
        let placement: PlacementOrder = serde_json::from_value(sent_from_the_viewer())
            .expect("el recuadro del visor tiene decimales");

        let placed = placement.placement().expect("cabe en la pagina");
        assert_eq!(
            (
                placed.rect.lower_left_x,
                placed.rect.lower_left_y,
                placed.rect.upper_right_x,
                placed.rect.upper_right_y
            ),
            (48, 179, 250, 260)
        );
    }

    fn sent_from_the_viewer() -> serde_json::Value {
        serde_json::json!({
            "page": 1,
            "pages": { "only": [1] },
            "pageCount": 3,
            "mediaBox": [0.0, 0.0, 595.276, 841.89],
            "rotation": 0,
            "rect": [47.7218, 179.1376722440945, 250.1, 259.9],
        })
    }

    fn order_placed_on(pages: serde_json::Value, page_count: u32) -> PlacementOrder {
        let mut sent = sent_from_the_viewer();
        sent["pages"] = pages;
        sent["pageCount"] = serde_json::json!(page_count);
        serde_json::from_value(sent).expect("la orden del visor")
    }

    #[test]
    fn refuses_a_destination_the_document_does_not_have_before_calling_the_bridge() {
        let failure = order_placed_on(serde_json::json!({ "only": [99] }), 3)
            .placement()
            .expect_err("un documento de tres paginas no tiene la 99");

        assert_eq!(failure.situation, "pageOutOfDocument");
        assert!(failure.detail.contains("99"), "{}", failure.detail);
    }

    #[test]
    fn refuses_a_drag_page_the_document_does_not_have() {
        let mut sent = sent_from_the_viewer();
        sent["page"] = serde_json::json!(9);
        sent["pages"] = serde_json::json!("all");
        let order: PlacementOrder = serde_json::from_value(sent).expect("la orden del visor");

        assert_eq!(
            order.placement().expect_err("la 9 no existe").situation,
            "pageOutOfDocument"
        );
    }

    #[test]
    fn carries_the_page_set_through_to_the_placement() {
        let placed = order_placed_on(serde_json::json!("all"), 3)
            .placement()
            .expect("cabe y existe");
        assert_eq!(placed.pages, PageSet::All);

        let placed = order_placed_on(serde_json::json!({ "only": [3, 1] }), 3)
            .placement()
            .expect("cabe y existe");
        assert_eq!(placed.pages, PageSet::only([1, 3]).expect("no esta vacio"));
    }
}
