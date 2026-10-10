//! Una tarjeta que se cuelga en `C_Initialize`: la llamada no vuelve, y la tarjeta sana no se ve afectada.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use fake_pkcs11::FakeCard;

const HANG_PATIENCE: Duration = Duration::from_millis(500);
const HEALTHY_PATIENCE: Duration = Duration::from_secs(120);
const TIMED_OUT: &str = "C_Initialize no vuelve a tiempo";

fn initialize_within(card: &FakeCard, patience: Duration) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();
    let module = card.module();
    thread::spawn(move || {
        let outcome = Pkcs11::new(module)
            .map_err(|error| format!("el módulo falso no se carga: {error}"))
            .and_then(|context| {
                context
                    .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
                    .map_err(|error| format!("C_Initialize falla: {error}"))
            });
        let _ = sender.send(outcome);
    });
    match receiver.recv_timeout(patience) {
        Ok(outcome) => outcome,
        Err(RecvTimeoutError::Timeout) => Err(TIMED_OUT.to_owned()),
        Err(RecvTimeoutError::Disconnected) => {
            Err("el hilo de C_Initialize entra en pánico".to_owned())
        }
    }
}

#[test]
fn a_hanging_card_never_returns_from_c_initialize() {
    let card = FakeCard::new()
        .and_then(FakeCard::hanging_on_initialize)
        .expect("la tarjeta falsa deberia montarse");

    assert_eq!(
        initialize_within(&card, HANG_PATIENCE),
        Err(TIMED_OUT.to_owned())
    );
}

#[test]
fn a_healthy_card_returns_from_c_initialize() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    assert_eq!(initialize_within(&card, HEALTHY_PATIENCE), Ok(()));
}
