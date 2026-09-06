use super::*;

#[test]
fn a_home_without_a_local_ca_is_the_first_boot_and_not_a_failure() {
    assert_eq!(Stage::of(None), Stage::Absent);
}

#[test]
fn a_local_ca_with_years_left_is_simply_serving() {
    assert_eq!(Stage::of(Some(700)), Stage::Serving);
}

#[test]
fn the_next_local_ca_goes_in_months_before_the_current_one_expires() {
    assert_eq!(Stage::of(Some(OVERLAP_DAYS)), Stage::Serving);
    assert_eq!(Stage::of(Some(OVERLAP_DAYS - 1)), Stage::Overlapping);
    assert_eq!(Stage::of(Some(1)), Stage::Overlapping);
}

#[test]
fn a_local_ca_that_ran_out_is_expired_and_not_overlapping() {
    assert_eq!(Stage::of(Some(0)), Stage::Expired);
    assert_eq!(Stage::of(Some(-40)), Stage::Expired);
}

#[test]
fn nothing_is_ever_repaired_in_the_middle_of_an_errand() {
    for stage in [
        Stage::Absent,
        Stage::Serving,
        Stage::Overlapping,
        Stage::Expired,
    ] {
        for next in [NextCa::None, NextCa::Waiting] {
            assert_eq!(work_at(Moment::MidErrand, stage, next), Work::Nothing);
        }
    }
}

#[test]
fn the_first_boot_makes_a_local_ca_and_so_does_an_expired_one_with_no_successor() {
    assert_eq!(
        work_at(Moment::Startup, Stage::Absent, NextCa::None),
        Work::MakeOneAndInstallIt
    );
    assert_eq!(
        work_at(Moment::Startup, Stage::Expired, NextCa::None),
        Work::MakeOneAndInstallIt
    );
}

#[test]
fn a_local_ca_that_still_serves_is_installed_but_never_remade() {
    assert_eq!(
        work_at(Moment::Startup, Stage::Serving, NextCa::None),
        Work::InstallTheOneWeHave
    );
}

#[test]
fn the_overlap_installs_the_next_one_without_asking_for_the_current_one_back() {
    assert_eq!(
        work_at(Moment::Startup, Stage::Overlapping, NextCa::None),
        Work::MakeTheNextAndInstallItToo
    );
}

#[test]
fn the_next_local_ca_is_made_once_and_then_only_installed() {
    assert_eq!(
        work_at(Moment::Startup, Stage::Overlapping, NextCa::Waiting),
        Work::InstallBothOfThem
    );
}

#[test]
fn an_expired_local_ca_with_a_successor_waiting_hands_over_instead_of_starting_again() {
    assert_eq!(
        work_at(Moment::Startup, Stage::Expired, NextCa::Waiting),
        Work::PromoteTheNextOne
    );
}

#[test]
fn the_notice_never_shows_up_in_the_middle_of_an_errand() {
    let pending = PendingNotice::after_installing();

    assert_eq!(pending.mid_errand(), None);
    assert!(pending.is_pending());
}

#[test]
fn the_notice_comes_out_once_when_the_errand_ends() {
    let mut pending = PendingNotice::after_installing();

    assert_eq!(
        pending.when_the_errand_ends(),
        Some(Notice::RestartTheBrowser)
    );
    assert_eq!(pending.when_the_errand_ends(), None);
}

#[test]
fn nothing_installed_means_nothing_to_say() {
    let mut pending = PendingNotice::none();

    assert!(!pending.is_pending());
    assert_eq!(pending.when_the_errand_ends(), None);
}
