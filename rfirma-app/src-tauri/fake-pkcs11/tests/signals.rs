//! La tarjeta de señales: las tres siempre leíbles, también en un proceso nuevo, y un login por cada firma.

mod common;

use cryptoki::error::{Error, RvError};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, ObjectClass, ObjectHandle};
use cryptoki::session::{Session, UserType};
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;

use common::{log_in, process, session, signals};

const WRONG_PIN: &str = "87654321";

fn signing_key(session: &Session) -> ObjectHandle {
    session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(b"KprivFirmaDigital".to_vec()),
        ])
        .unwrap()[0]
}

fn context_login(session: &Session) {
    session
        .login(
            UserType::ContextSpecific,
            Some(&AuthPin::new(FakeCard::PIN.into())),
        )
        .unwrap();
}

fn user_session(card: &FakeCard) -> (cryptoki::context::Pkcs11, Session) {
    let (context, slot) = process(card);
    let session = session(&context, slot);
    session
        .login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())))
        .unwrap();
    (context, session)
}

fn fail_in_a_process(card: &FakeCard) {
    let (context, slot) = process(card);
    let _ = log_in(&context, slot, WRONG_PIN);
}

fn signals_in_a_process(card: &FakeCard) -> Vec<&'static str> {
    let (context, slot) = process(card);
    signals(&context, slot)
}

#[test]
fn each_signal_is_readable_in_a_new_process_when_its_moment_comes() {
    let card = FakeCard::new().unwrap().signals_profile().unwrap();
    assert!(signals_in_a_process(&card).is_empty());

    fail_in_a_process(&card);
    assert_eq!(signals_in_a_process(&card), ["COUNT_LOW"]);
    fail_in_a_process(&card);
    assert_eq!(signals_in_a_process(&card), ["COUNT_LOW", "FINAL_TRY"]);
    fail_in_a_process(&card);
    assert_eq!(signals_in_a_process(&card), ["COUNT_LOW", "LOCKED"]);
}

#[test]
fn the_signing_key_demands_a_context_login_and_the_other_key_does_not() {
    let card = FakeCard::new().unwrap().signals_profile().unwrap();
    let (_context, session) = user_session(&card);

    let flag = |label: &[u8]| {
        let key = session
            .find_objects(&[
                Attribute::Class(ObjectClass::PRIVATE_KEY),
                Attribute::Label(label.to_vec()),
            ])
            .unwrap()[0];
        session
            .get_attributes(key, &[AttributeType::AlwaysAuthenticate])
            .unwrap()
    };
    assert_eq!(
        flag(b"KprivFirmaDigital"),
        [Attribute::AlwaysAuthenticate(true)]
    );
    assert!(flag(b"KprivAutenticacion").is_empty());
}

#[test]
fn signing_fails_without_the_context_login_and_enters_with_it() {
    let card = FakeCard::new().unwrap().signals_profile().unwrap();
    let (_context, session) = user_session(&card);
    session
        .sign_init(&Mechanism::Sha256RsaPkcs, signing_key(&session))
        .unwrap();
    session.sign_update(b"datos").unwrap();

    let refused = session.sign_final();
    assert!(matches!(
        refused,
        Err(Error::Pkcs11(RvError::UserNotLoggedIn, _))
    ));
    context_login(&session);
    assert!(session.sign_final().is_ok());
}

#[test]
fn every_signature_needs_its_own_context_login() {
    let card = FakeCard::new().unwrap().signals_profile().unwrap();
    let (_context, session) = user_session(&card);
    let key = signing_key(&session);
    session.sign_init(&Mechanism::Sha256RsaPkcs, key).unwrap();
    context_login(&session);
    session.sign_update(b"primero").unwrap();
    session.sign_final().unwrap();

    session.sign_init(&Mechanism::Sha256RsaPkcs, key).unwrap();
    session.sign_update(b"segundo").unwrap();

    assert!(matches!(
        session.sign_final(),
        Err(Error::Pkcs11(RvError::UserNotLoggedIn, _))
    ));
}

#[test]
fn the_dnie_profile_signs_without_a_context_login() {
    let card = FakeCard::new().unwrap();
    let (_context, session) = user_session(&card);

    let signature = session.sign(&Mechanism::Sha256RsaPkcs, signing_key(&session), b"datos");

    assert!(signature.is_ok());
}

#[test]
fn a_context_login_with_a_wrong_pin_spends_a_try() {
    let card = FakeCard::new().unwrap().signals_profile().unwrap();
    let (_context, session) = user_session(&card);
    session
        .sign_init(&Mechanism::Sha256RsaPkcs, signing_key(&session))
        .unwrap();

    let refused = session.login(
        UserType::ContextSpecific,
        Some(&AuthPin::new(WRONG_PIN.into())),
    );

    assert!(matches!(
        refused,
        Err(Error::Pkcs11(RvError::PinIncorrect, _))
    ));
    assert_eq!(card.tries_left(), 2);
}
