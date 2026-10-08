//! Lo que comparten las pruebas: arrancar el módulo de una tarjeta como lo haría un proceso nuevo.

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::session::Session;
use cryptoki::slot::Slot;
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
