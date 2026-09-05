//! **El recuadro que pide la sede**, leído de sus `extraParams` y nada más
//! (ID-282, ID-283, ID-284).
//!
//! Aquí se decide una sola cosa: si la petición de la sede lleva recuadro, si
//! no lo lleva, o si lo que pide no se atiende. Lo que **no** se hace es
//! convertir coordenadas, y esa ausencia es la decisión del ID-282.
//!
//! # Los dos caminos del recuadro no comparten conversión
//!
//! - **Lo elige la persona**, arrastrando sobre el visor: hay que aplicar el
//!   `T⁻¹` de la `/Rotate` ([`crate::signing::placement`]), porque ella apuntó
//!   a un punto de la pantalla y el recuadro tiene que caer ahí.
//! - **Lo elige la sede**: los `signaturePositionOnPage*` cruzan al puente
//!   **crudos**. Medido en el tag `v1.9.2`: la conversión del diálogo de
//!   colocación de AutoFirma (`SignPdfUiPanel.java:1049-1081`) es una regla de
//!   tres sobre `getPageSizeWithRotation`, no hay ningún `T⁻¹` y la CropBox no
//!   aparece en todo el proyecto; los números que manda una sede están
//!   ajustados contra ese comportamiento, así que «arreglárselos» mueve el
//!   recuadro fuera de donde ella lo puso.
//!
//! Unificar los dos caminos es lo natural y **rompe uno de los dos**. En el de
//! la sede, «hacerlo bien» y «ser conforme» no son lo mismo, y manda ser
//! conforme.
//!
//! # Quién resuelve las páginas contadas desde el final
//!
//! El puente, y sólo él: `normalizePage` ya aplica `page + totalPages + 1`, así
//! que `-1` es la última sin que Rust toque nada. Resolverlas también aquí
//! daría la página equivocada (ID-284).
//!
//! # Cómo se honra el recuadro
//!
//! No emitiéndolo. Los `extraParams` de la sede son la base y los ajustes de
//! rFirma se escriben encima ([`crate::app::policies::merged_with`]), así que
//! una firma de sede se configura **sin colocación**
//! ([`crate::signing::SignatureConfig::placement`] a `None`) y las claves que
//! mandó la sede llegan al puente tal y como vinieron. Pasarlas por
//! [`crate::signing::Placement`] y volver a serializarlas sería un viaje de ida
//! y vuelta que pierde por el camino `signaturePage`, los rangos y los índices
//! negativos.

use std::collections::BTreeMap;

use super::codes::{Parameter, SafCode};
use super::refusal::Refusal;

/// Las cuatro esquinas del recuadro, en la convención del original
/// (`PdfExtraParams`).
const CORNERS: [&str; 4] = [
    "signaturePositionOnPageLowerLeftX",
    "signaturePositionOnPageLowerLeftY",
    "signaturePositionOnPageUpperRightX",
    "signaturePositionOnPageUpperRightY",
];

/// La página del recuadro, en singular (`PdfExtraParams.SIGNATURE_PAGE`).
const PAGE: &str = "signaturePage";

/// Y en plural, que gana al singular cuando vienen las dos
/// (`PdfUtil.getPages:699-703`).
const PAGES: &str = "signaturePages";

/// La bandera con la que la sede dice si el recuadro es obligatorio
/// (`PdfExtraParams.VISIBLE_SIGNATURE`).
const VISIBLE_SIGNATURE: &str = "visibleSignature";

/// El valor de esa bandera que hace el recuadro **obligatorio**.
const WANT: &str = "want";

/// El primer elemento de la lista de páginas que pide **añadir una en blanco**
/// al documento (`PdfUtil.java:711-713`).
const APPEND: &str = "append";

/// **El recuadro que pide la sede**, ya decidido.
///
/// No lleva coordenadas dentro a propósito: lo que la sede mandó ya está en sus
/// `extraParams` y de ahí sale sin pasar por ningún tipo nuestro. Esto dice
/// **qué va a ocurrir**, no cuánto mide nada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteVisibleSignature {
    /// La sede mandó posición y página: se firma con recuadro, y sus
    /// coordenadas cruzan al puente crudas.
    PlacedByTheSite,
    /// No hay recuadro que colocar. Es en lo que queda «la persona ha declinado
    /// colocar» cuando no hay ni visor ni diálogo: `optional` firma invisible
    /// y un `visibleAppearance=custom` sin datos estampa el aspecto por
    /// omisión.
    Declined,
}

/// Qué recuadro pide la sede, o por qué no se le atiende.
///
/// Se lee sobre los `extraParams` **ya expandidos**, que es donde el original
/// mira también (`ProtocolInvocationLauncherSign.java:502-505`, después de
/// `ExtraParamsProcessor.expandProperties`).
///
/// Dos negativas, y ninguna espera a la persona:
///
/// - **`SAF_43`** cuando `visibleSignature=want` y no vienen las cuatro
///   esquinas **y** una página. Es la misma condición que
///   `checkShowRubricDialogIsCanceled` (`ProtocolInvocationLauncherSign.java:960`-`979`)
///   comprueba tras cancelarse el diálogo, y el mismo código que el original
///   acaba emitiendo (ID-283). rFirma no enseña el diálogo, así que llega aquí
///   directamente.
/// - **`SAF_03` nombrando `properties`** cuando la sede **puso el recuadro** y
///   la lista de páginas que manda empieza por `append`: añadir una página en
///   blanco es modificar el documento antes de firmarlo (ID-284). No se
///   contesta `SAF_43` para que la sede pueda distinguir las dos cosas: una es
///   un recuadro que falta, y la otra un valor que rFirma no atiende.
///
/// El orden importa: **la negativa del `append` va detrás del recuadro**,
/// porque sin las cuatro esquinas el original tampoco añade página ninguna
/// (`PdfSessionManager.java:383-390` sólo llama a `PdfUtil.getPages` dentro del
/// `if (signaturePositionOnPage != null)`, y ese rectángulo lo exige entero
/// `PdfUtil.getPositionOnPage:467-470`). Un `visibleSignature=optional` con
/// `signaturePages=append` y sin esquinas no toca el documento: firma
/// invisible, como pide el original.
pub fn visible_signature_of(
    params: &BTreeMap<String, String>,
) -> Result<SiteVisibleSignature, Refusal> {
    if the_site_placed_the_box(params) {
        refuse_an_appended_page(params)?;
        return Ok(SiteVisibleSignature::PlacedByTheSite);
    }

    if the_site_makes_it_mandatory(params) {
        return Err(Refusal::new(
            SafCode::VisibleSignature,
            format!(
                "'{VISIBLE_SIGNATURE}={WANT}' exige recuadro y la peticion no trae posicion y \
                 pagina: no hay donde colocarlo"
            ),
        ));
    }

    Ok(SiteVisibleSignature::Declined)
}

/// Las cuatro esquinas y una página, que es lo que el original entiende por
/// «el área ya viene puesta» (`existsAreaAttributes`).
///
/// Mira que las claves **estén**, sin leer lo que traen dentro, exactamente
/// como el original: quien interpreta esos valores es el puente, y adelantarse
/// a él aquí sería tener dos opiniones sobre el mismo texto.
fn the_site_placed_the_box(params: &BTreeMap<String, String>) -> bool {
    CORNERS.iter().all(|corner| params.contains_key(*corner))
        && (params.contains_key(PAGE) || params.contains_key(PAGES))
}

/// Si la sede declaró el recuadro obligatorio.
///
/// Sin recortar espacios, que es como lo lee el original:
/// `checkShowRubricDialogIsCanceled` compara con `equalsIgnoreCase` a secas
/// (`ProtocolInvocationLauncherSign.java:960`-`979`), así que un
/// `visibleSignature=" WANT "` allí no es obligatorio y la firma sale
/// invisible. Recortar aquí endurecería una negativa que el original no hace.
fn the_site_makes_it_mandatory(params: &BTreeMap<String, String>) -> bool {
    params
        .get(VISIBLE_SIGNATURE)
        .is_some_and(|value| value.eq_ignore_ascii_case(WANT))
}

/// La negativa del `append`, en **la** clave que manda.
///
/// Se mira una sola: `signaturePages` si está, y sólo si no está,
/// `signaturePage`. Es la cuenta exacta del original
/// (`PdfUtil.getPages:699-703` lee la singular únicamente cuando la plural
/// falta), así que `signaturePages=2` con `signaturePage=append` firma en la
/// página 2 sin añadir nada, igual que allí.
///
/// **Sólo cuenta como primer elemento de la lista**, que es el único sitio
/// donde el original añade la página (`PdfUtil.java:711-713`). El propio
/// diálogo de AutoFirma emite `"3,append"` (`SignPdfUiPanel.java:530-534`), que
/// ahí dentro no crea ninguna página: rechazarlo sería inventarse una
/// incompatibilidad con las sedes que copian esa cadena.
fn refuse_an_appended_page(params: &BTreeMap<String, String>) -> Result<(), Refusal> {
    let key = if params.contains_key(PAGES) {
        PAGES
    } else {
        PAGE
    };
    let Some(value) = params.get(key) else {
        return Ok(());
    };
    if first_of(value).eq_ignore_ascii_case(APPEND) {
        return Err(Refusal::about(
            Parameter::Properties,
            format!(
                "'{key}={value}' pide anadir una pagina en blanco al documento, y eso es \
                 modificarlo antes de firmarlo"
            ),
        ));
    }
    Ok(())
}

/// El primer elemento de una lista separada por comas, ya sin espacios.
fn first_of(value: &str) -> &str {
    value.split(',').next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests {
    use super::{visible_signature_of, SiteVisibleSignature};
    use crate::protocol::{Parameter, SafCode};
    use std::collections::BTreeMap;

    /// **Grada A**: se leen unos `extraParams` y sale una decisión (TD-53).
    fn asked(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    /// Las cuatro esquinas que manda una sede, tal y como vienen.
    const CORNERS: [(&str, &str); 4] = [
        ("signaturePositionOnPageLowerLeftX", "100"),
        ("signaturePositionOnPageLowerLeftY", "100"),
        ("signaturePositionOnPageUpperRightX", "300"),
        ("signaturePositionOnPageUpperRightY", "180"),
    ];

    /// Las cuatro esquinas más lo que se le añada.
    fn placed(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut pairs = CORNERS.to_vec();
        pairs.extend_from_slice(extra);
        asked(&pairs)
    }

    /// **ID-282**: con posición y página, se firma con recuadro.
    #[test]
    fn a_position_and_a_page_from_the_site_are_honoured() {
        let asked = placed(&[("signaturePages", "1")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    /// Y la página vale en singular, que es la clave que llevan las sedes
    /// viejas.
    #[test]
    fn the_page_of_the_box_also_counts_when_it_comes_in_the_singular_key() {
        let asked = placed(&[("signaturePage", "2")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    /// **ID-282**: sin posición, `optional` firma invisible y no molesta a
    /// nadie.
    #[test]
    fn an_optional_visible_signature_without_a_place_to_put_it_is_signed_invisible() {
        let asked = asked(&[("visibleSignature", "optional")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    /// **ID-283**: y `want` sin posición se rechaza con el código que el
    /// original ya tiene para esto.
    #[test]
    fn a_mandatory_visible_signature_without_a_place_to_put_it_is_refused() {
        let refusal = visible_signature_of(&asked(&[("visibleSignature", "want")]))
            .expect_err("no hay donde colocar el recuadro");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
        assert_eq!(refusal.blame(), None);
    }

    /// La bandera se lee como la lee el original: sin distinguir mayúsculas.
    #[test]
    fn the_mandatory_flag_is_read_without_telling_capitals_apart() {
        let refusal = visible_signature_of(&asked(&[("visibleSignature", "WANT")]))
            .expect_err("sigue siendo obligatorio");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    /// Y **con espacios alrededor deja de serlo**, que es también como la lee
    /// el original: `equalsIgnoreCase` sin `trim()`.
    #[test]
    fn the_mandatory_flag_padded_with_spaces_is_not_mandatory_either_in_the_original() {
        assert_eq!(
            visible_signature_of(&asked(&[("visibleSignature", " WANT ")])),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    /// Con posición y página, `want` es una firma más: no hay nada que
    /// reclamar.
    #[test]
    fn a_mandatory_visible_signature_that_came_placed_is_just_a_signature() {
        let asked = placed(&[("signaturePages", "1"), ("visibleSignature", "want")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    /// Las cuatro esquinas **sin página** no son un sitio donde colocar el
    /// recuadro: es la misma cuenta que hace `existsAreaAttributes`.
    #[test]
    fn corners_without_a_page_are_not_a_place_to_put_the_box() {
        let refusal = {
            let mut asked = placed(&[("visibleSignature", "want")]);
            asked.remove("signaturePages");
            visible_signature_of(&asked).expect_err("faltaba la pagina")
        };

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    /// Y una esquina que falta tampoco: el recuadro son las cuatro.
    #[test]
    fn three_corners_are_not_a_box() {
        let mut asked = placed(&[("signaturePages", "1")]);
        asked.remove("signaturePositionOnPageUpperRightY");

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    /// **ID-282**: un aspecto a medida del que no vienen los datos es el
    /// aspecto por omisión, y se firma igual.
    #[test]
    fn a_custom_appearance_with_nothing_to_customise_is_the_appearance_by_default() {
        let asked = asked(&[
            ("visibleAppearance", "custom"),
            ("visibleSignature", "optional"),
        ]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    /// **ID-284**: los índices contados desde el final se atienden, y quien los
    /// resuelve es el puente: aquí no se toca ninguno.
    #[test]
    fn pages_counted_from_the_end_are_resolved_by_the_bridge_and_by_nobody_else() {
        for pages in ["-1", "all", "1-3,-3--1"] {
            let asked = placed(&[("signaturePages", pages)]);

            assert_eq!(
                visible_signature_of(&asked),
                Ok(SiteVisibleSignature::PlacedByTheSite),
                "'{pages}' es gramática del puente y cruza entera"
            );
        }
    }

    /// **ID-284**: añadir una página en blanco es modificar el documento antes
    /// de firmarlo, y eso se rechaza nombrando el parámetro que lo pidió.
    #[test]
    fn a_page_appended_to_the_document_is_refused_because_signing_never_modifies_it() {
        for key in ["signaturePages", "signaturePage"] {
            let refusal = visible_signature_of(&placed(&[(key, "append")]))
                .expect_err("no se anaden paginas");

            assert_eq!(refusal.code(), SafCode::Params);
            assert_eq!(refusal.blame(), Some(Parameter::Properties));
        }
    }

    /// Sin recuadro puesto no hay página que añadir: el original tampoco mira
    /// la lista, así que `optional` firma invisible en vez de rechazarse.
    #[test]
    fn an_append_without_the_box_placed_adds_no_page_and_signs_invisible() {
        let asked = asked(&[
            ("visibleSignature", "optional"),
            ("signaturePages", "append"),
        ]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    /// Y con `want` sin esquinas la negativa sigue siendo la del recuadro que
    /// falta, no la del `append`.
    #[test]
    fn an_append_without_the_box_placed_is_still_the_missing_box_refusal() {
        let refusal = visible_signature_of(&asked(&[
            ("visibleSignature", "want"),
            ("signaturePages", "append"),
        ]))
        .expect_err("no hay donde colocar el recuadro");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    /// El plural gana al singular, también para el `append`: `PdfUtil.getPages`
    /// ni lee `signaturePage` cuando viene `signaturePages`.
    #[test]
    fn the_plural_key_wins_so_an_append_in_the_singular_one_is_never_read() {
        let asked = placed(&[("signaturePages", "2"), ("signaturePage", "append")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    /// Pero el `append` que el diálogo del original emite **detrás** de una
    /// página no añade nada, y rechazarlo sería inventarse una
    /// incompatibilidad.
    #[test]
    fn the_append_that_the_original_writes_after_a_page_never_adds_one() {
        let asked = placed(&[("signaturePages", "3,append")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    /// Una petición que no dice nada del recuadro no lleva recuadro.
    #[test]
    fn a_request_that_says_nothing_about_the_box_carries_no_box() {
        assert_eq!(
            visible_signature_of(&BTreeMap::new()),
            Ok(SiteVisibleSignature::Declined)
        );
    }
}
