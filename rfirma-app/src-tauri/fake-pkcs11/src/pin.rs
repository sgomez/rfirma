//! El PIN de la tarjeta: contador entre procesos y señales leídas de él.

use std::path::Path;

use cryptoki_sys::{
    CKF_USER_PIN_COUNT_LOW, CKF_USER_PIN_FINAL_TRY, CKF_USER_PIN_LOCKED, CKR_OK, CKR_PIN_INCORRECT,
    CKR_PIN_LEN_RANGE, CKR_PIN_LOCKED, CK_FLAGS, CK_RV,
};

use crate::card::{read_tries_left, write_tries_left, MAX_TRIES, PIN};

pub(crate) const MIN_PIN_LEN: usize = 8;
pub(crate) const MAX_PIN_LEN: usize = 16;

pub(crate) fn verify(dir: &Path, pin: Option<&[u8]>) -> CK_RV {
    let Some(pin) = pin.filter(|pin| (MIN_PIN_LEN..=MAX_PIN_LEN).contains(&pin.len())) else {
        return CKR_PIN_LEN_RANGE;
    };
    let tries = read_tries_left(dir);
    if tries == 0 {
        return CKR_PIN_LOCKED;
    }
    if pin == PIN.as_bytes() {
        write_tries_left(dir, MAX_TRIES);
        return CKR_OK;
    }
    write_tries_left(dir, tries - 1);
    CKR_PIN_INCORRECT
}

/// Las señales del PIN, leídas del contador.
pub(crate) fn standing_signals(tries_left: u8) -> CK_FLAGS {
    match tries_left {
        0 => CKF_USER_PIN_COUNT_LOW | CKF_USER_PIN_LOCKED,
        1 => CKF_USER_PIN_COUNT_LOW | CKF_USER_PIN_FINAL_TRY,
        MAX_TRIES => 0,
        _ => CKF_USER_PIN_COUNT_LOW,
    }
}
