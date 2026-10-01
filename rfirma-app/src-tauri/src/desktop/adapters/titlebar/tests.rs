use std::path::Path;

use super::super::views::TitlebarRecentView;
use super::*;

#[test]
fn a_state_equal_to_the_last_applied_is_ignored() {
    assert_eq!(decide(Some(&1), &1, false), Arrival::Ignore);
    assert_eq!(decide(Some(&1), &1, true), Arrival::Ignore);
}

#[test]
fn a_different_state_without_an_open_menu_is_applied() {
    assert_eq!(decide(Some(&1), &2, false), Arrival::Apply);
    assert_eq!(decide(None, &1, false), Arrival::Apply);
}

#[test]
fn a_different_state_with_an_open_menu_is_deferred() {
    assert_eq!(decide(Some(&1), &2, true), Arrival::Defer);
}

#[test]
fn closing_the_menu_applies_only_the_last_deferred_state() {
    let mut pacing = Pacing::new();
    assert_eq!(pacing.arrive(1, false), Some(1));
    assert_eq!(pacing.arrive(2, true), None);
    assert_eq!(pacing.arrive(3, true), None);

    assert_eq!(pacing.menu_closed(), Some(3));
    assert_eq!(pacing.menu_closed(), None);
    assert_eq!(pacing.arrive(3, false), None);
}

#[test]
fn closing_the_menu_applies_nothing_when_the_last_arrival_matched_the_applied_state() {
    let mut pacing = Pacing::new();
    assert_eq!(pacing.arrive(1, false), Some(1));
    assert_eq!(pacing.arrive(2, true), None);
    assert_eq!(pacing.arrive(1, true), None);

    assert_eq!(pacing.menu_closed(), None);
}

#[test]
fn closing_the_menu_with_nothing_deferred_applies_nothing() {
    let mut pacing = Pacing::new();
    assert_eq!(pacing.arrive(1, false), Some(1));

    assert_eq!(pacing.menu_closed(), None);
}

fn recent(signed: bool, found: bool, location: Option<&str>) -> TitlebarRecentView {
    TitlebarRecentView {
        path: "opaque-id".into(),
        name: "informe.pdf".into(),
        location: location.map(Into::into),
        signed,
        found,
    }
}

#[test]
fn a_recent_without_location_still_arrives_and_leaves_the_second_line_empty() {
    let arrived: TitlebarRecentView = serde_json::from_value(serde_json::json!({
        "path": "opaque-id",
        "name": "informe.pdf",
        "folder": "Documentos",
        "signed": false,
        "found": true,
    }))
    .unwrap();

    assert_eq!(arrived.location, None);
    assert_eq!(second_line(&arrived, "No se encuentra"), "");
    assert_eq!(announcement(&arrived, "No se encuentra"), "informe.pdf");
}

#[test]
fn the_second_line_of_a_found_recent_is_its_location() {
    let found = recent(false, true, Some("~/Documentos"));
    assert_eq!(second_line(&found, "No se encuentra"), "~/Documentos");
}

#[test]
fn the_second_line_of_a_missing_recent_says_it_is_not_found() {
    let missing = recent(false, false, Some("~/Documentos"));
    assert_eq!(second_line(&missing, "No se encuentra"), "No se encuentra");
}

#[test]
fn a_row_announces_its_name_whether_it_is_signed_and_its_second_line() {
    let signed = recent(true, true, Some("~/Documentos"));
    assert_eq!(
        announcement(&signed, "No se encuentra"),
        "informe.pdf \u{2713}, ~/Documentos"
    );
    let missing = recent(false, false, None);
    assert_eq!(
        announcement(&missing, "No se encuentra"),
        "informe.pdf, No se encuentra"
    );
}

#[test]
fn the_tooltip_of_a_recent_is_its_absolute_path_with_home_expanded() {
    let inside = recent(false, true, Some("~/Documentos/2026"));
    assert_eq!(
        tooltip(&inside, Some(Path::new("/home/ana"))).as_deref(),
        Some("/home/ana/Documentos/2026/informe.pdf")
    );
    let at_home = recent(false, false, Some("~/"));
    assert_eq!(
        tooltip(&at_home, Some(Path::new("/home/ana"))).as_deref(),
        Some("/home/ana/informe.pdf")
    );
    let outside = recent(false, true, Some("/srv/actas"));
    assert_eq!(
        tooltip(&outside, Some(Path::new("/home/ana"))).as_deref(),
        Some("/srv/actas/informe.pdf")
    );
}

#[test]
fn a_recent_without_location_has_no_tooltip() {
    assert_eq!(
        tooltip(&recent(false, true, None), Some(Path::new("/home/ana"))),
        None
    );
}
