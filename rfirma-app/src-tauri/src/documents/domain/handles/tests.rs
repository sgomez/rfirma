use super::*;

#[test]
fn a_handle_is_thirty_two_hexadecimal_digits() {
    let handle = mint();

    assert_eq!(handle.len(), 32);
    assert!(handle
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn two_handles_are_never_the_same() {
    assert_ne!(mint(), mint());
}

#[test]
fn the_fallback_keeps_the_shape_and_the_difference() {
    let first = minted_without_the_system_csprng();
    let second = minted_without_the_system_csprng();

    assert_eq!(first.len(), 32);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    assert_ne!(first, second);
}
