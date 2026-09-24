use super::{
    mark_the_area, the_mandatory_area_was_cancelled, visible_signature_of, IfCancelled,
    SiteVisibleSignature,
};
use crate::site::domain::protocol::{Parameter, SafCode};
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
fn an_optional_visible_signature_without_an_area_asks_the_person_and_cancelling_signs_invisible() {
    let asked = asked(&[("visibleSignature", "optional")]);

    assert_eq!(
        visible_signature_of(&asked),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::SignsInvisible
        ))
    );
}

#[test]
fn a_mandatory_visible_signature_without_an_area_asks_the_person_and_cancelling_refuses() {
    assert_eq!(
        visible_signature_of(&asked(&[("visibleSignature", "want")])),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::Refuses
        ))
    );
}

#[test]
fn a_cancelled_mandatory_area_is_answered_with_the_visible_signature_code() {
    let refusal = the_mandatory_area_was_cancelled();

    assert_eq!(refusal.code(), SafCode::VisibleSignature);
    assert_eq!(refusal.blame(), None);
}

#[test]
fn the_flag_is_read_without_telling_capitals_apart() {
    assert_eq!(
        visible_signature_of(&asked(&[("visibleSignature", "WANT")])),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::Refuses
        ))
    );
    assert_eq!(
        visible_signature_of(&asked(&[("visibleSignature", "Optional")])),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::SignsInvisible
        ))
    );
}

#[test]
fn a_flag_that_is_neither_want_nor_optional_opens_no_dialog() {
    assert_eq!(
        visible_signature_of(&asked(&[("visibleSignature", "yes")])),
        Ok(SiteVisibleSignature::Declined)
    );
}

#[test]
fn the_mandatory_flag_padded_with_spaces_is_not_mandatory_either_in_the_original() {
    assert_eq!(
        visible_signature_of(&asked(&[("visibleSignature", " WANT ")])),
        Ok(SiteVisibleSignature::Declined)
    );
}

#[test]
fn a_wanted_or_optional_area_that_came_in_the_request_is_still_asked_and_cancelling_keeps_it() {
    for flag in ["want", "optional"] {
        let asked = placed(&[("signaturePages", "1"), ("visibleSignature", flag)]);

        assert_eq!(
            visible_signature_of(&asked),
            Ok(SiteVisibleSignature::MarkedByThePerson(
                IfCancelled::SignsWhereTheSiteSaid
            )),
            "con '{flag}'"
        );
    }
}

#[test]
fn corners_without_a_page_are_not_a_place_to_put_the_box() {
    let mut asked = placed(&[("visibleSignature", "want")]);
    asked.remove("signaturePages");

    assert_eq!(
        visible_signature_of(&asked),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::Refuses
        ))
    );
}

#[test]
fn the_area_the_person_marks_replaces_every_key_of_the_request_area() {
    let mut asked = placed(&[("signaturePage", "2"), ("signaturePages", "3")]);

    mark_the_area(
        &mut asked,
        [
            ("signaturePages".to_owned(), "1".to_owned()),
            (
                "signaturePositionOnPageLowerLeftX".to_owned(),
                "10".to_owned(),
            ),
            (
                "signaturePositionOnPageLowerLeftY".to_owned(),
                "20".to_owned(),
            ),
            (
                "signaturePositionOnPageUpperRightX".to_owned(),
                "130".to_owned(),
            ),
            (
                "signaturePositionOnPageUpperRightY".to_owned(),
                "54".to_owned(),
            ),
        ],
    );

    assert_eq!(
        asked,
        self::asked(&[
            ("signaturePages", "1"),
            ("signaturePositionOnPageLowerLeftX", "10"),
            ("signaturePositionOnPageLowerLeftY", "20"),
            ("signaturePositionOnPageUpperRightX", "130"),
            ("signaturePositionOnPageUpperRightY", "54"),
        ])
    );
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
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::SignsInvisible
        ))
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
        let refusal =
            visible_signature_of(&placed(&[(key, "append")])).expect_err("no se anaden paginas");

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
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::SignsInvisible
        ))
    );
}

#[test]
fn an_append_without_the_box_placed_is_still_a_mandatory_area_to_mark() {
    assert_eq!(
        visible_signature_of(&asked(&[
            ("visibleSignature", "want"),
            ("signaturePages", "append"),
        ])),
        Ok(SiteVisibleSignature::MarkedByThePerson(
            IfCancelled::Refuses
        ))
    );
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
