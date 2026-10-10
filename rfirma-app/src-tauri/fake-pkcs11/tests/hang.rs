//! Una tarjeta que se cuelga en `C_Initialize`: la llamada no vuelve, y la tarjeta sana no se ve afectada.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use fake_pkcs11::FakeCard;

const PATIENCE: Duration = Duration::from_millis(500);

fn initialize_returns_in_time(card: &FakeCard) -> bool {
    let (sender, receiver) = mpsc::channel();
    let module = card.module();
    thread::spawn(move || {
        let context = Pkcs11::new(module).expect("el módulo falso deberia cargarse");
        let _ = context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK));
        let _ = sender.send(());
    });
    receiver.recv_timeout(PATIENCE).is_ok()
}

#[test]
fn a_hanging_card_never_returns_from_c_initialize() {
    let card = FakeCard::new()
        .and_then(FakeCard::hanging_on_initialize)
        .expect("la tarjeta falsa deberia montarse");

    assert!(!initialize_returns_in_time(&card));
}

#[test]
fn a_healthy_card_returns_from_c_initialize() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    assert!(initialize_returns_in_time(&card));
}
