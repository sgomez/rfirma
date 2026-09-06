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
        StoreSecret::TypedOnScreen {
            attempts_left: None
        }
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
fn the_attempts_left_are_empty_because_pkcs11_never_counts_them() {
    let StoreSecret::TypedOnScreen { attempts_left } = StoreSecret::of_token(true, false)
    else {
        panic!("un almacen con sesion y sin teclado pide el secreto por pantalla");
    };
    assert_eq!(attempts_left, None);
}

#[test]
fn the_two_secrets_that_can_be_asked_for_are_admitted() {
    assert_eq!(
        StoreSecret::NotNeeded.admitted(),
        Ok(StoreSecret::NotNeeded)
    );
    let on_screen = StoreSecret::TypedOnScreen {
        attempts_left: None,
    };
    assert_eq!(on_screen.admitted(), Ok(on_screen));
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
