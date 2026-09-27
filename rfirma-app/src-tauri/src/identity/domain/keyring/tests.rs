use super::*;

#[test]
fn generates_a_long_random_pin() {
    let pin = generate_pin();

    assert_eq!(pin.len(), PIN_RANDOM_BYTES * 2);
}

#[test]
fn generates_a_different_pin_every_time() {
    let first = generate_pin();
    let second = generate_pin();

    assert_ne!(first, second);
}

#[test]
fn no_keyring_and_pin_missing_are_told_apart() {
    assert_ne!(KeyringError::NoKeyring, KeyringError::PinMissing);
}
