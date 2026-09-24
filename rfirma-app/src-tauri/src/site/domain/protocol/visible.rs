//! Recuadro de firma visible solicitado por la sede (ADR-0019).

use std::collections::BTreeMap;

use super::codes::{Parameter, SafCode};
use super::refusal::{Refusal, RefusalSituation};

const CORNERS: [&str; 4] = [
    "signaturePositionOnPageLowerLeftX",
    "signaturePositionOnPageLowerLeftY",
    "signaturePositionOnPageUpperRightX",
    "signaturePositionOnPageUpperRightY",
];

const PAGE: &str = "signaturePage";
const PAGES: &str = "signaturePages";
const VISIBLE_SIGNATURE: &str = "visibleSignature";
const RUBRIC_IMAGE: &str = "signatureRubricImage";
const WANT: &str = "want";
const OPTIONAL: &str = "optional";
const APPEND: &str = "append";

/// Recuadro de firma visible solicitado por la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteVisibleSignature {
    /// La sede especificó posición y página para la firma visible.
    PlacedByTheSite,
    /// La petición no incluye recuadro a colocar.
    Declined,
    /// La persona marca el área antes de consentir, y cancelar hace lo que dice el valor.
    MarkedByThePerson(IfCancelled),
}

/// Qué pasa si la persona cancela el diálogo del área.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IfCancelled {
    /// `visibleSignature=want` sin área en la petición: la sede recibe `SAF_43`.
    Refuses,
    /// La petición trae el área: se firma en ella.
    SignsWhereTheSiteSaid,
    /// `visibleSignature=optional` sin área en la petición: se firma invisible.
    SignsInvisible,
}

/// Evalúa si la sede solicita recuadro de firma visible o si se rechaza la petición.
pub fn visible_signature_of(
    params: &BTreeMap<String, String>,
) -> Result<SiteVisibleSignature, Refusal> {
    let placed = the_site_placed_the_box(params);
    if placed {
        refuse_an_appended_page(params)?;
    }

    let visible = match (the_person_is_asked(params), placed) {
        (Some(_), true) => {
            SiteVisibleSignature::MarkedByThePerson(IfCancelled::SignsWhereTheSiteSaid)
        }
        (Some(true), false) => SiteVisibleSignature::MarkedByThePerson(IfCancelled::Refuses),
        (Some(false), false) => {
            SiteVisibleSignature::MarkedByThePerson(IfCancelled::SignsInvisible)
        }
        (None, true) => SiteVisibleSignature::PlacedByTheSite,
        (None, false) => SiteVisibleSignature::Declined,
    };
    Ok(visible)
}

/// La negativa que recibe la sede cuando la persona cancela un área obligatoria.
pub fn the_mandatory_area_was_cancelled() -> Refusal {
    Refusal::new(
        SafCode::VisibleSignature,
        format!(
            "'{VISIBLE_SIGNATURE}={WANT}' exige recuadro, la peticion no trae posicion y pagina \
             y la persona ha cancelado el dialogo del area"
        ),
    )
}

/// Sustituye el área de la petición por la que marcó la persona.
pub fn mark_the_area(
    params: &mut BTreeMap<String, String>,
    area: impl IntoIterator<Item = (String, String)>,
) {
    for key in CORNERS.iter().chain(&[PAGE, PAGES]) {
        params.remove(*key);
    }
    params.extend(area);
}

/// Olvida el recuadro y la rúbrica que declaró la sede: solo los lee un firmador PDF.
pub fn forget_the_box(params: &mut BTreeMap<String, String>) {
    for key in CORNERS
        .iter()
        .chain(&[PAGE, PAGES, VISIBLE_SIGNATURE, RUBRIC_IMAGE])
    {
        params.remove(*key);
    }
}

fn the_site_placed_the_box(params: &BTreeMap<String, String>) -> bool {
    CORNERS.iter().all(|corner| params.contains_key(*corner))
        && (params.contains_key(PAGE) || params.contains_key(PAGES))
}

/// `Some(true)` con `want`, `Some(false)` con `optional`, y `None` sin diálogo que abrir.
fn the_person_is_asked(params: &BTreeMap<String, String>) -> Option<bool> {
    let value = params.get(VISIBLE_SIGNATURE)?;
    if value.eq_ignore_ascii_case(WANT) {
        Some(true)
    } else if value.eq_ignore_ascii_case(OPTIONAL) {
        Some(false)
    } else {
        None
    }
}

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
        )
        .because(RefusalSituation::AppendedSignaturePage));
    }
    Ok(())
}

fn first_of(value: &str) -> &str {
    value.split(',').next().unwrap_or_default().trim()
}

#[cfg(test)]
mod tests;
