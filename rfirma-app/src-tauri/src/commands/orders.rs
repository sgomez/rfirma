//! **Lo que la ventana manda**: las órdenes tal y como llegan deserializadas.
//!
//! Van aparte de los tipos de salida de [`super::views`] porque entran en vez
//! de salir: aquí no hay ninguna guarda del ADR-0011 que valga —lo que llega no
//! puede filtrar una ruta del anfitrión, porque la ventana no tiene ninguna—.
//! Lo que sí hay es la única traducción que estos tipos hacen: la geometría del
//! visor a la del PDF.

use serde::Deserialize;

use crate::commands::Failure;
use crate::signing::{MediaBox, Page, PageSet, Placement, Rotation, UserSpaceRect};

/// Lo que la ventana ha marcado en las casillas del recuadro.
///
/// La casilla «DNI» ya no está: el dato viaja dentro del `CN` del firmante y
/// lo tapa la máscara al componer el texto. En su sitio entra «Emisor», que
/// hasta ahora solo se veía en el desplegable de certificados.
///
/// `serde(default)` porque una casilla que la ventana no mande vale «sin
/// marcar»: el recuadro sale con un dato de menos, que es mucho menos malo que
/// rechazar la orden entera y no firmar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VisibleFieldsOrder {
    pub signer_name: bool,
    pub issuer: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// Dónde ha caído el recuadro, tal como lo sabe el visor.
///
/// La `MediaBox` y la `/Rotate` las trae la ventana porque quien tiene abierto
/// el PDF es `pdf.js`: el backend **no lee PDFs**, y ponerle un analizador para
/// releer lo que el visor ya sabe sería una segunda opinión sobre la misma
/// página.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementOrder {
    /// La página **sobre la que se arrastró**, 1-based como la numera
    /// `pdf.js`. No es el destino —eso es [`PlacementOrder::pages`]—: es la
    /// página cuya `MediaBox` y cuya `/Rotate` describen las coordenadas que
    /// vienen en `rect`.
    pub page: u32,
    /// **En qué páginas se estampa** (ID-91).
    pub pages: PageSet,
    /// Cuántas páginas tiene el documento, según el visor, que es quien lo
    /// tiene abierto.
    ///
    /// Viaja con la orden porque **el backend no lee PDFs** y sin ella no
    /// puede validar el destino: `PdfUtil.getPages` recorta en silencio y
    /// firma en la última con cara de éxito (ID-94).
    pub page_count: u32,
    /// La `MediaBox` de la página del arrastre: `[x0, y0, x1, y1]`.
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
    /// La colocación en puntos PAdES, o la negativa si el destino no existe o
    /// el recuadro se sale de la página.
    ///
    /// Es público porque quien lo llama es el caso de uso que arma la
    /// configuración de firma ([`crate::app::signing`]), y no una orden.
    ///
    /// **Las dos negativas se comprueban antes de llamar al puente** y no hay
    /// otro sitio donde comprobarlas: iText recorta un recuadro que se salga
    /// (ID-22) y `PdfUtil.getPages` recorta un destino que no exista (ID-94),
    /// los dos en silencio y devolviendo éxito.
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

/// La situación con la que la ventana pinta un destino que el documento no
/// tiene (ID-29, ID-94).
const PAGE_OUT_OF_DOCUMENT: &str = "pageOutOfDocument";

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
    ///
    /// **Falta en el trámite de sede, y por eso es opcional** (ID-282): ahí no
    /// hay visor sobre el que arrastrar nada, el recuadro lo coloca la sede en
    /// sus propios `extraParams` y esos cruzan al puente crudos. Aplicarles el
    /// `T⁻¹` de este camino los movería fuera de donde ella los puso; ver
    /// [`crate::protocol::visible`].
    ///
    /// **Sin `#[serde(default)]` a propósito**: la ventana siempre lo manda, y
    /// que faltara en el JSON no puede degradarse a firma invisible en
    /// silencio —es la misma degradación que el ID-22 rechaza en el recuadro—.
    /// Que falte es un error de deserialización, y así se ve.
    pub placement: Option<PlacementOrder>,
    pub fields: VisibleFieldsOrder,
    /// El motivo. Vacío es «sin motivo».
    pub reason: String,
    /// La fecha y hora, ya formateadas.
    pub signed_at: String,
    /// La rúbrica en JPEG y Base64, ya normalizada por [`crate::rubric`].
    pub rubric: Option<String>,
    /// El idioma en el que se componen las etiquetas del recuadro.
    pub language: String,
    /// Que la persona ya ha dicho que sí a cofirmar un PDF con **firmas que
    /// rFirma no sabe leer** (ID-297, ID-301).
    ///
    /// Llega desde la ventana porque la pregunta es suya —el aviso se enseña
    /// antes de pedir el PIN— y `#[serde(default)]` porque una orden que no lo
    /// diga es una orden que nadie ha consentido: **por omisión, no**.
    #[serde(default)]
    pub allow_unregistered_signatures: bool,
}

#[cfg(test)]
mod tests {
    use super::PlacementOrder;
    use crate::signing::PageSet;

    /// La ventana manda el recuadro en espacio de usuario **tal cual sale de
    /// `convertToPdfPoint`**, y eso son fracciones: `pdf.js` invierte una
    /// matriz, no cuenta puntos enteros.
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

    /// Lo que el visor manda de verdad, con el conjunto de páginas y el número
    /// de páginas del documento.
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

    /// **TD-30**: el destino fuera de rango **no llega al puente**. Se prueba
    /// aquí porque el puente no lo rechazaría: firmaría en la última página y
    /// devolvería éxito.
    #[test]
    fn refuses_a_destination_the_document_does_not_have_before_calling_the_bridge() {
        let failure = order_placed_on(serde_json::json!({ "only": [99] }), 3)
            .placement()
            .expect_err("un documento de tres paginas no tiene la 99");

        assert_eq!(failure.situation, "pageOutOfDocument");
        assert!(failure.detail.contains("99"), "{}", failure.detail);
    }

    /// La página del arrastre también es un destino: es donde acaba el
    /// recuadro cuando el conjunto es «todas».
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
