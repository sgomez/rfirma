//! Lo que comparten las pruebas: arrancar el módulo de una tarjeta como lo haría un proceso nuevo.

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::error::{Error, RvError};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;
use fake_pkcs11::FakeCard;

/// El módulo de la tarjeta, recién cargado e inicializado, con su única ranura.
pub fn process(card: &FakeCard) -> (Pkcs11, Slot) {
    let context = Pkcs11::new(card.module()).expect("el módulo falso deberia cargarse");
    context
        .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
        .expect("C_Initialize deberia aceptar el bloqueo del sistema");
    let slots = context
        .get_slots_with_token()
        .expect("C_GetSlotList deberia responder");
    assert_eq!(slots.len(), 1, "el lector falso tiene una sola ranura");
    (context, slots[0])
}

/// Una sesión de solo lectura en la ranura de la tarjeta.
pub fn session(context: &Pkcs11, slot: Slot) -> Session {
    context
        .open_ro_session(slot)
        .expect("C_OpenSession deberia abrir una sesión")
}

/// Un `C_Login` de usuario en una sesión nueva, con el código PKCS#11 que devuelve.
#[allow(dead_code)]
pub fn log_in(context: &Pkcs11, slot: Slot, pin: &str) -> Result<(), RvError> {
    match session(context, slot).login(UserType::User, Some(&AuthPin::new(pin.into()))) {
        Ok(()) => Ok(()),
        Err(Error::Pkcs11(rv, _)) => Err(rv),
        Err(other) => panic!("C_Login deberia devolver un código PKCS#11: {other}"),
    }
}

/// Las señales del PIN que ve este proceso en `C_GetTokenInfo`.
#[allow(dead_code)]
pub fn signals(context: &Pkcs11, slot: Slot) -> Vec<&'static str> {
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
