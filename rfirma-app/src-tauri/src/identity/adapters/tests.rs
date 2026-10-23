use super::*;

#[test]
fn the_desktop_stores_of_linux_end_with_the_dnie() {
    assert!(desktop_stores()
        .last()
        .is_some_and(|store| store.is_the_dnie()));
}
