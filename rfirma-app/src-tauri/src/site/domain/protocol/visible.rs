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
const WANT: &str = "want";
const APPEND: &str = "append";

/// Recuadro de firma visible solicitado por la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteVisibleSignature {
    /// La sede especificó posición y página para la firma visible.
    PlacedByTheSite,
    /// La petición no incluye recuadro a colocar.
    Declined,
}

/// Evalúa si la sede solicita recuadro de firma visible o si se rechaza la petición.
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

fn the_site_placed_the_box(params: &BTreeMap<String, String>) -> bool {
    CORNERS.iter().all(|corner| params.contains_key(*corner))
        && (params.contains_key(PAGE) || params.contains_key(PAGES))
}

fn the_site_makes_it_mandatory(params: &BTreeMap<String, String>) -> bool {
    params
        .get(VISIBLE_SIGNATURE)
        .is_some_and(|value| value.eq_ignore_ascii_case(WANT))
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
