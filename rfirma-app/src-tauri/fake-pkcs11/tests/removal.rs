//! La retirada de la tarjeta: desde la llamada N, todo lo que la toca falla como sin tarjeta.

mod common;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};
use cryptoki::session::UserType;
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;

use common::process;

const OPEN_SESSION_CALL: usize = 5;

fn is_rv<T: std::fmt::Debug>(result: Result<T, Error>, expected: RvError) -> bool {
    matches!(result, Err(Error::Pkcs11(rv, _)) if rv == expected)
}

#[test]
fn calls_before_the_removal_still_work() {
    let card = FakeCard::new().unwrap().removed_from_call(1000).unwrap();
    let (context, slot) = process(&card);

    let session = context.open_ro_session(slot).unwrap();

    assert!(session
        .login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())))
        .is_ok());
}

#[test]
fn from_the_removal_on_the_session_calls_fail_as_without_a_card() {
    let card = FakeCard::new()
        .unwrap()
        .removed_from_call(OPEN_SESSION_CALL)
        .unwrap();
    let (context, slot) = process(&card);

    assert!(is_rv(
        context.open_ro_session(slot),
        RvError::TokenNotPresent
    ));
    assert!(is_rv(
        context.get_token_info(slot),
        RvError::TokenNotPresent
    ));
}

#[test]
fn an_open_session_loses_its_card_when_it_is_removed() {
    let login_call = 6;
    let card = FakeCard::new()
        .unwrap()
        .removed_from_call(login_call)
        .unwrap();
    let (context, slot) = process(&card);
    let session = context.open_ro_session(slot).unwrap();

    let login = session.login(UserType::User, Some(&AuthPin::new(FakeCard::PIN.into())));

    assert!(is_rv(login, RvError::DeviceRemoved));
    assert_eq!(card.tries_left(), 3);
}

#[test]
fn the_reader_stays_but_lists_no_card() {
    let card = FakeCard::new().unwrap().removed_from_call(1).unwrap();
    let context = Pkcs11::new(card.module()).unwrap();
    context
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .unwrap();

    assert!(context.get_slots_with_token().unwrap().is_empty());
    let readers = context.get_all_slots().unwrap();
    assert_eq!(readers.len(), 1);
    assert!(!context.get_slot_info(readers[0]).unwrap().token_present());
}
