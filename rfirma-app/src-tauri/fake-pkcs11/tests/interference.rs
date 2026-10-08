//! La interferencia de otro programa: el login de una sesión abierta antes falla sin gastar un intento.

mod common;

use cryptoki::error::{Error, RvError};
use cryptoki::session::UserType;
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;

use common::{process, session};

fn pin() -> AuthPin {
    AuthPin::new(FakeCard::PIN.into())
}

#[test]
fn an_interfered_login_says_not_logged_in_and_keeps_the_tries() {
    let card = FakeCard::new().unwrap().interfered().unwrap();
    let (context, slot) = process(&card);
    let stale = session(&context, slot);

    let refused = stale.login(UserType::User, Some(&pin()));

    assert!(matches!(
        refused,
        Err(Error::Pkcs11(RvError::UserNotLoggedIn, _))
    ));
    assert_eq!(card.tries_left(), 3);
    assert_eq!(
        card.calls_to("C_Login"),
        [format!(
            "C_Login CKU_USER \"{}\" -> CKR_USER_NOT_LOGGED_IN",
            FakeCard::PIN
        )]
    );
}

#[test]
fn the_same_pin_enters_in_a_new_session() {
    let card = FakeCard::new().unwrap().interfered().unwrap();
    let (context, slot) = process(&card);
    let stale = session(&context, slot);
    assert!(stale.login(UserType::User, Some(&pin())).is_err());

    let fresh = session(&context, slot);

    assert!(fresh.login(UserType::User, Some(&pin())).is_ok());
}

#[test]
fn without_the_indication_the_login_enters_at_once() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);

    assert!(session(&context, slot)
        .login(UserType::User, Some(&pin()))
        .is_ok());
}
