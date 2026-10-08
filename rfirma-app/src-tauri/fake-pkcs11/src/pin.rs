//! El PIN del DNIe por OpenSC: ninguna señal antes de un fallo, y el intento que bloquea dice «incorrecto».

use std::path::Path;

use cryptoki_sys::{
    CKF_USER_PIN_COUNT_LOW, CKF_USER_PIN_FINAL_TRY, CKF_USER_PIN_LOCKED, CKR_OK, CKR_PIN_INCORRECT,
    CKR_PIN_LEN_RANGE, CKR_PIN_LOCKED, CK_FLAGS, CK_RV,
};

use crate::card::{read_tries_left, write_tries_left, MAX_TRIES, PIN};

pub(crate) const MIN_PIN_LEN: usize = 8;
pub(crate) const MAX_PIN_LEN: usize = 16;

/// Lo que responde la tarjeta a un PIN, y las señales que deja en el proceso que lo envió.
pub(crate) struct Attempt {
    pub(crate) rv: CK_RV,
    /// `None` si el PIN no llegó a la tarjeta y las señales del proceso no cambian.
    pub(crate) signals: Option<CK_FLAGS>,
}

pub(crate) fn verify(dir: &Path, pin: Option<&[u8]>) -> Attempt {
    let Some(pin) = pin.filter(|pin| (MIN_PIN_LEN..=MAX_PIN_LEN).contains(&pin.len())) else {
        return Attempt {
            rv: CKR_PIN_LEN_RANGE,
            signals: None,
        };
    };
    let tries = read_tries_left(dir);
    if tries == 0 {
        return answered(CKR_PIN_LOCKED, 0);
    }
    if pin == PIN.as_bytes() {
        write_tries_left(dir, MAX_TRIES);
        return answered(CKR_OK, 0);
    }
    write_tries_left(dir, tries - 1);
    answered(CKR_PIN_INCORRECT, signals_after_failure(tries - 1))
}

fn answered(rv: CK_RV, signals: CK_FLAGS) -> Attempt {
    Attempt {
        rv,
        signals: Some(signals),
    }
}

/// Lo que OpenSC saca del `63Cx` del `VERIFY` fallido: nunca `COUNT_LOW`.
fn signals_after_failure(tries_left: u8) -> CK_FLAGS {
    match tries_left {
        0 => CKF_USER_PIN_LOCKED,
        1 => CKF_USER_PIN_FINAL_TRY,
        _ => 0,
    }
}

/// Las señales de una tarjeta que las da siempre, leídas del contador y no del último fallo.
pub(crate) fn standing_signals(tries_left: u8) -> CK_FLAGS {
    match tries_left {
        0 => CKF_USER_PIN_COUNT_LOW | CKF_USER_PIN_LOCKED,
        1 => CKF_USER_PIN_COUNT_LOW | CKF_USER_PIN_FINAL_TRY,
        MAX_TRIES => 0,
        _ => CKF_USER_PIN_COUNT_LOW,
    }
}
