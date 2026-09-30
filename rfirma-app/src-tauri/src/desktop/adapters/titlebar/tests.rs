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
