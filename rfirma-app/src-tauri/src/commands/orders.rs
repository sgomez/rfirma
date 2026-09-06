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
mod tests;
