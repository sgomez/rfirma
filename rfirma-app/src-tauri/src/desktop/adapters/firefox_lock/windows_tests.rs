use std::os::windows::fs::OpenOptionsExt;

use super::*;

#[test]
fn a_profile_with_no_lock_file_is_not_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");

    assert!(!firefox_is_running(profile.path()));
}

#[test]
fn a_profile_whose_lock_file_nobody_holds_is_not_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    std::fs::write(profile.path().join("parent.lock"), b"")
        .expect("deberia poder crearse el fichero de cerrojo");

    assert!(!firefox_is_running(profile.path()));
}

#[test]
fn a_profile_whose_lock_file_is_held_without_sharing_is_running() {
    let profile = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
    let held = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(profile.path().join("parent.lock"))
        .expect("se abre sin compartir, como Firefox");

    assert!(firefox_is_running(profile.path()));
    drop(held);
    assert!(!firefox_is_running(profile.path()));
}
