//! El PIN de la tarjeta falsa se porta como el del DNIe medido: contador entre procesos y señales solo tras un fallo.

mod common;

use cryptoki::context::Pkcs11;
use cryptoki::error::{Error, RvError};
use cryptoki::object::{Attribute, ObjectClass};
use cryptoki::session::UserType;
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;

use common::{process, session};

const WRONG_PIN: &str = "87654321";

/// Un proceso que arranca el módulo, envía un PIN y se va: el contador queda en la tarjeta.
fn log_in_once(card: &FakeCard, pin: &str) -> Result<(), RvError> {
    let (context, slot) = process(card);
    log_in(&context, slot, pin)
}

fn log_in(context: &Pkcs11, slot: Slot, pin: &str) -> Result<(), RvError> {
    match session(context, slot).login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => Ok(()),
        Err(Error::Pkcs11(rv, _)) => Err(rv),
        Err(other) => panic!("C_Login deberia devolver un código PKCS#11: {other}"),
    }
}

/// Las señales del PIN que ve este proceso en `C_GetTokenInfo`.
fn signals(context: &Pkcs11, slot: Slot) -> Vec<&'static str> {
    let info = context.get_token_info(slot).unwrap();
    [
        (info.user_pin_count_low(), "COUNT_LOW"),
        (info.user_pin_final_try(), "FINAL_TRY"),
        (info.user_pin_locked(), "LOCKED"),
    ]
    .into_iter()
    .filter_map(|(lit, name)| lit.then_some(name))
    .collect()
}

#[test]
fn each_wrong_pin_spends_one_try_that_the_card_remembers() {
    let card = FakeCard::new().unwrap();

    assert_eq!(log_in_once(&card, WRONG_PIN), Err(RvError::PinIncorrect));
    assert_eq!(card.tries_left(), 2);
    assert_eq!(log_in_once(&card, WRONG_PIN), Err(RvError::PinIncorrect));
    assert_eq!(card.tries_left(), 1);
}

#[test]
fn the_right_pin_restores_the_tries() {
    let card = FakeCard::with_tries_left(1).unwrap();

    assert_eq!(log_in_once(&card, FakeCard::PIN), Ok(()));
    assert_eq!(card.tries_left(), 3);
}

#[test]
fn without_a_failure_there_is_no_signal() {
    let card = FakeCard::with_tries_left(1).unwrap();
    let (context, slot) = process(&card);

    assert!(signals(&context, slot).is_empty());
}

#[test]
fn a_first_failure_lights_no_signal() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);

    assert_eq!(
        log_in(&context, slot, WRONG_PIN),
        Err(RvError::PinIncorrect)
    );
    assert!(signals(&context, slot).is_empty());
}

#[test]
fn the_second_failure_lights_final_try_only_in_its_process() {
    let card = FakeCard::new().unwrap();
    {
        let (context, slot) = process(&card);
        assert_eq!(
            log_in(&context, slot, WRONG_PIN),
            Err(RvError::PinIncorrect)
        );
        assert_eq!(
            log_in(&context, slot, WRONG_PIN),
            Err(RvError::PinIncorrect)
        );
        assert_eq!(signals(&context, slot), ["FINAL_TRY"]);
    }

    let (context, slot) = process(&card);
    assert!(signals(&context, slot).is_empty());
}

#[test]
fn the_failure_that_locks_says_incorrect_with_locked() {
    let card = FakeCard::with_tries_left(1).unwrap();
    let (context, slot) = process(&card);

    assert_eq!(
        log_in(&context, slot, WRONG_PIN),
        Err(RvError::PinIncorrect)
    );
    assert_eq!(signals(&context, slot), ["LOCKED"]);
    assert_eq!(card.tries_left(), 0);
}

#[test]
fn a_locked_card_refuses_even_the_right_pin_and_signals_nothing() {
    let card = FakeCard::with_tries_left(1).unwrap();
    let (context, slot) = process(&card);
    assert_eq!(
        log_in(&context, slot, WRONG_PIN),
        Err(RvError::PinIncorrect)
    );

    assert_eq!(
        log_in(&context, slot, FakeCard::PIN),
        Err(RvError::PinLocked)
    );
    assert!(signals(&context, slot).is_empty());
    drop(context);
    assert_eq!(log_in_once(&card, FakeCard::PIN), Err(RvError::PinLocked));
}

#[test]
fn a_locked_card_lists_like_a_healthy_one() {
    let card = FakeCard::locked().unwrap();
    let (context, slot) = process(&card);

    let certificates = session(&context, slot)
        .find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])
        .unwrap();
    assert_eq!(certificates.len(), 3);
    assert!(signals(&context, slot).is_empty());
}

#[test]
fn a_pin_without_value_is_refused_without_spending_a_try() {
    let card = FakeCard::new().unwrap();
    let (context, slot) = process(&card);

    let blind = session(&context, slot).login(UserType::User, None);
    assert!(matches!(blind, Err(Error::Pkcs11(RvError::PinLenRange, _))));
    assert_eq!(card.tries_left(), 3);
}

#[test]
fn the_card_records_every_login_with_its_pin_and_answer() {
    let card = FakeCard::new().unwrap();

    let _ = log_in_once(&card, WRONG_PIN);
    let _ = log_in_once(&card, FakeCard::PIN);

    assert_eq!(
        card.calls_to("C_Login"),
        [
            format!("C_Login CKU_USER \"{WRONG_PIN}\" -> CKR_PIN_INCORRECT"),
            format!("C_Login CKU_USER \"{}\" -> CKR_OK", FakeCard::PIN),
        ]
    );
}
