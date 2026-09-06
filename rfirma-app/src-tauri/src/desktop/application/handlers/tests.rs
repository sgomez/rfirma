use super::*;
use crate::desktop::domain::error::Situation;

#[test]
fn inside_the_sandbox_nothing_can_be_known() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let view = who_handles(Channel::Flatpak, &directory.path().join("mimeapps.list"));

    assert!(!view.available);
    assert!(view.handlers.is_empty());
    assert_eq!(view.current, None);
}

#[test]
fn outside_the_sandbox_the_written_choice_is_the_one_shown() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let list = directory.path().join("mimeapps.list");
    chosen(Channel::Native, &list, OUR_DESKTOP_FILE).expect("deberia escribirse");

    let view = who_handles(Channel::Native, &list);

    assert!(view.available);
    assert_eq!(view.current.as_deref(), Some(OUR_DESKTOP_FILE));
}

#[test]
fn our_own_launcher_crosses_with_the_answer() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let view = who_handles(Channel::Native, &directory.path().join("mimeapps.list"));

    assert_eq!(view.ours, OUR_DESKTOP_FILE);
}

#[test]
fn choosing_inside_the_sandbox_fails_with_its_own_situation() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    let failure = chosen(
        Channel::Flatpak,
        &directory.path().join("mimeapps.list"),
        OUR_DESKTOP_FILE,
    )
    .expect_err("dentro del sandbox no se escribe");

    assert_eq!(failure.situation(), Situation::NotAvailableInsideTheSandbox);
    assert!(!failure.detail().is_empty());
}
