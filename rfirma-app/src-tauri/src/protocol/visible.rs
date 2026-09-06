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
mod tests {
    use super::{visible_signature_of, SiteVisibleSignature};
    use crate::protocol::{Parameter, SafCode};
    use std::collections::BTreeMap;

    fn asked(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    const CORNERS: [(&str, &str); 4] = [
        ("signaturePositionOnPageLowerLeftX", "100"),
        ("signaturePositionOnPageLowerLeftY", "100"),
        ("signaturePositionOnPageUpperRightX", "300"),
        ("signaturePositionOnPageUpperRightY", "180"),
    ];

    fn placed(extra: &[(&str, &str)]) -> BTreeMap<String, String> {
        let mut pairs = CORNERS.to_vec();
        pairs.extend_from_slice(extra);
        asked(&pairs)
    }

    #[test]
    fn a_position_and_a_page_from_the_site_are_honoured() {
        let asked = placed(&[("signaturePages", "1")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    #[test]
    fn the_page_of_the_box_also_counts_when_it_comes_in_the_singular_key() {
        let asked = placed(&[("signaturePage", "2")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    #[test]
    fn an_optional_visible_signature_without_a_place_to_put_it_is_signed_invisible() {
        let asked = asked(&[("visibleSignature", "optional")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    #[test]
    fn a_mandatory_visible_signature_without_a_place_to_put_it_is_refused() {
        let refusal = visible_signature_of(&asked(&[("visibleSignature", "want")]))
            .expect_err("no hay donde colocar el recuadro");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
        assert_eq!(refusal.blame(), None);
    }

    #[test]
    fn the_mandatory_flag_is_read_without_telling_capitals_apart() {
        let refusal = visible_signature_of(&asked(&[("visibleSignature", "WANT")]))
            .expect_err("sigue siendo obligatorio");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    #[test]
    fn the_mandatory_flag_padded_with_spaces_is_not_mandatory_either_in_the_original() {
        assert_eq!(
            visible_signature_of(&asked(&[("visibleSignature", " WANT ")])),
            Ok(SiteVisibleSignature::Declined)
        );
    }

    #[test]
    fn a_mandatory_visible_signature_that_came_placed_is_just_a_signature() {
        let asked = placed(&[("signaturePages", "1"), ("visibleSignature", "want")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    #[test]
    fn corners_without_a_page_are_not_a_place_to_put_the_box() {
        let refusal = {
            let mut asked = placed(&[("visibleSignature", "want")]);
            asked.remove("signaturePages");
            visible_signature_of(&asked).expect_err("faltaba la pagina")
        };

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    #[test]
    fn three_corners_are_not_a_box() {
        let mut asked = placed(&[("signaturePages", "1")]);
        asked.remove("signaturePositionOnPageUpperRightY");

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::Declined)
        );
    }

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

    #[test]
    fn a_page_appended_to_the_document_is_refused_because_signing_never_modifies_it() {
        for key in ["signaturePages", "signaturePage"] {
            let refusal = visible_signature_of(&placed(&[(key, "append")]))
                .expect_err("no se anaden paginas");

            assert_eq!(refusal.code(), SafCode::Params);
            assert_eq!(refusal.blame(), Some(Parameter::Properties));
        }
    }

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

    #[test]
    fn an_append_without_the_box_placed_is_still_the_missing_box_refusal() {
        let refusal = visible_signature_of(&asked(&[
            ("visibleSignature", "want"),
            ("signaturePages", "append"),
        ]))
        .expect_err("no hay donde colocar el recuadro");

        assert_eq!(refusal.code(), SafCode::VisibleSignature);
    }

    #[test]
    fn the_plural_key_wins_so_an_append_in_the_singular_one_is_never_read() {
        let asked = placed(&[("signaturePages", "2"), ("signaturePage", "append")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    #[test]
    fn the_append_that_the_original_writes_after_a_page_never_adds_one() {
        let asked = placed(&[("signaturePages", "3,append")]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::PlacedByTheSite)
        );
    }

    #[test]
    fn a_request_that_says_nothing_about_the_box_carries_no_box() {
        assert_eq!(
            visible_signature_of(&BTreeMap::new()),
            Ok(SiteVisibleSignature::Declined)
        );
    }
}
