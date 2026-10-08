//! Cómo se presentan la biblioteca, el lector y la tarjeta: los textos y las banderas de un DNIe por OpenSC.

use cryptoki_sys::{
    CKF_HW, CKF_HW_SLOT, CKF_LOGIN_REQUIRED, CKF_REMOVABLE_DEVICE, CKF_RNG, CKF_SIGN,
    CKF_TOKEN_INITIALIZED, CKF_TOKEN_PRESENT, CKF_USER_PIN_INITIALIZED, CK_FLAGS, CK_INFO,
    CK_MECHANISM_INFO, CK_SLOT_INFO, CK_TOKEN_INFO, CK_ULONG, CK_UNAVAILABLE_INFORMATION,
    CK_VERSION,
};

use crate::pin::{MAX_PIN_LEN, MIN_PIN_LEN};

const VERSION: CK_VERSION = CK_VERSION {
    major: 2,
    minor: 40,
};
const DEVICE_VERSION: CK_VERSION = CK_VERSION { major: 0, minor: 1 };

pub(crate) fn library() -> CK_INFO {
    CK_INFO {
        cryptokiVersion: VERSION,
        manufacturerID: padded("rfirma"),
        flags: 0,
        libraryDescription: padded("Modulo PKCS#11 falso"),
        libraryVersion: DEVICE_VERSION,
    }
}

pub(crate) fn slot() -> CK_SLOT_INFO {
    CK_SLOT_INFO {
        slotDescription: padded("Lector falso de DNIe"),
        manufacturerID: padded("rfirma"),
        flags: CKF_TOKEN_PRESENT | CKF_REMOVABLE_DEVICE | CKF_HW_SLOT,
        hardwareVersion: DEVICE_VERSION,
        firmwareVersion: DEVICE_VERSION,
    }
}

/// La tarjeta, con las señales del PIN que tenga el proceso y sus sesiones abiertas.
pub(crate) fn token(pin_signals: CK_FLAGS, sessions: usize) -> CK_TOKEN_INFO {
    CK_TOKEN_INFO {
        label: padded("DNI electrónico"),
        manufacturerID: padded("DGP-FNMT"),
        model: padded("PKCS#15 emulated"),
        serialNumber: padded("FAKE0000000001"),
        flags: CKF_RNG
            | CKF_LOGIN_REQUIRED
            | CKF_USER_PIN_INITIALIZED
            | CKF_TOKEN_INITIALIZED
            | pin_signals,
        ulMaxSessionCount: 0,
        ulSessionCount: sessions as CK_ULONG,
        ulMaxRwSessionCount: 0,
        ulRwSessionCount: 0,
        ulMaxPinLen: MAX_PIN_LEN as CK_ULONG,
        ulMinPinLen: MIN_PIN_LEN as CK_ULONG,
        ulTotalPublicMemory: CK_UNAVAILABLE_INFORMATION,
        ulFreePublicMemory: CK_UNAVAILABLE_INFORMATION,
        ulTotalPrivateMemory: CK_UNAVAILABLE_INFORMATION,
        ulFreePrivateMemory: CK_UNAVAILABLE_INFORMATION,
        hardwareVersion: DEVICE_VERSION,
        firmwareVersion: DEVICE_VERSION,
        utcTime: padded(""),
    }
}

pub(crate) fn mechanism() -> CK_MECHANISM_INFO {
    CK_MECHANISM_INFO {
        ulMinKeySize: 1024,
        ulMaxKeySize: 2048,
        flags: CKF_HW | CKF_SIGN,
    }
}

/// Un texto PKCS#11: rellenado con espacios y sin cero final.
fn padded<const N: usize>(text: &str) -> [u8; N] {
    let mut field = [b' '; N];
    let bytes = text.as_bytes();
    field[..bytes.len()].copy_from_slice(bytes);
    field
}
