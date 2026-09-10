use super::*;

#[test]
fn a_store_that_asks_for_no_session_needs_no_secret() {
    assert_eq!(StoreSecret::of_token(false, false), StoreSecret::NotNeeded);
}

#[test]
fn a_store_that_asks_for_no_session_needs_no_secret_even_with_a_keypad() {
    assert_eq!(StoreSecret::of_token(false, true), StoreSecret::NotNeeded);
}

#[test]
fn a_store_that_asks_for_a_session_has_its_secret_typed_on_screen() {
    assert_eq!(
        StoreSecret::of_token(true, false),
        StoreSecret::TypedOnScreen
    );
}

#[test]
fn a_reader_with_its_own_keypad_is_told_apart_from_the_screen() {
    assert_eq!(
        StoreSecret::of_token(true, true),
        StoreSecret::TypedOnTheReaderKeypad
    );
}

#[test]
fn the_two_secrets_that_can_be_asked_for_are_admitted() {
    assert_eq!(
        StoreSecret::NotNeeded.admitted(),
        Ok(StoreSecret::NotNeeded)
    );
    assert_eq!(
        StoreSecret::TypedOnScreen.admitted(),
        Ok(StoreSecret::TypedOnScreen)
    );
}

#[test]
fn the_secret_of_a_reader_keypad_is_refused_instead_of_being_asked_on_screen() {
    assert_eq!(
        StoreSecret::TypedOnTheReaderKeypad.admitted(),
        Err(SecretOnTheReaderKeypad)
    );
}

#[test]
fn the_refusal_names_its_own_situation_and_says_why() {
    assert_eq!(
        SecretOnTheReaderKeypad.situation(),
        "secretOnTheReaderKeypad"
    );
    assert!(SecretOnTheReaderKeypad
        .to_string()
        .contains("teclado del lector"));
}

#[test]
fn a_module_asks_for_a_pin_and_every_store_that_is_a_file_asks_for_a_password() {
    assert_eq!(SecretName::of(StoreClass::Card), SecretName::Pin);

    for class in [
        StoreClass::Firefox,
        StoreClass::Chrome,
        StoreClass::Nssdb,
        StoreClass::Installed,
    ] {
        assert_eq!(SecretName::of(class), SecretName::Password);
    }
}
