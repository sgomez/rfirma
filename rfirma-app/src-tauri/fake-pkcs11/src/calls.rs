//! La línea del registro de cada llamada: función, argumentos que importan y el código devuelto, por su nombre.

use cryptoki_sys::*;

pub(crate) fn line(function: &str, detail: &str, rv: CK_RV) -> String {
    if detail.is_empty() {
        format!("{function} -> {}", rv_name(rv))
    } else {
        format!("{function} {detail} -> {}", rv_name(rv))
    }
}

pub(crate) fn user_name(user: CK_USER_TYPE) -> String {
    match user {
        CKU_SO => "CKU_SO".to_owned(),
        CKU_USER => "CKU_USER".to_owned(),
        CKU_CONTEXT_SPECIFIC => "CKU_CONTEXT_SPECIFIC".to_owned(),
        other => format!("{other:#x}"),
    }
}

pub(crate) fn pin_text(pin: Option<&[u8]>) -> String {
    match pin {
        None => "NULL".to_owned(),
        Some(pin) => format!("{:?}", String::from_utf8_lossy(pin)),
    }
}

const RV_NAMES: [(CK_RV, &str); 26] = [
    (CKR_OK, "CKR_OK"),
    (CKR_ARGUMENTS_BAD, "CKR_ARGUMENTS_BAD"),
    (CKR_ATTRIBUTE_SENSITIVE, "CKR_ATTRIBUTE_SENSITIVE"),
    (CKR_ATTRIBUTE_TYPE_INVALID, "CKR_ATTRIBUTE_TYPE_INVALID"),
    (CKR_BUFFER_TOO_SMALL, "CKR_BUFFER_TOO_SMALL"),
    (
        CKR_CRYPTOKI_ALREADY_INITIALIZED,
        "CKR_CRYPTOKI_ALREADY_INITIALIZED",
    ),
    (CKR_CRYPTOKI_NOT_INITIALIZED, "CKR_CRYPTOKI_NOT_INITIALIZED"),
    (CKR_DATA_LEN_RANGE, "CKR_DATA_LEN_RANGE"),
    (CKR_DEVICE_REMOVED, "CKR_DEVICE_REMOVED"),
    (CKR_FUNCTION_FAILED, "CKR_FUNCTION_FAILED"),
    (CKR_GENERAL_ERROR, "CKR_GENERAL_ERROR"),
    (CKR_KEY_HANDLE_INVALID, "CKR_KEY_HANDLE_INVALID"),
    (CKR_MECHANISM_INVALID, "CKR_MECHANISM_INVALID"),
    (CKR_OBJECT_HANDLE_INVALID, "CKR_OBJECT_HANDLE_INVALID"),
    (CKR_OPERATION_ACTIVE, "CKR_OPERATION_ACTIVE"),
    (
        CKR_OPERATION_NOT_INITIALIZED,
        "CKR_OPERATION_NOT_INITIALIZED",
    ),
    (CKR_PIN_INCORRECT, "CKR_PIN_INCORRECT"),
    (CKR_PIN_LEN_RANGE, "CKR_PIN_LEN_RANGE"),
    (CKR_PIN_LOCKED, "CKR_PIN_LOCKED"),
    (CKR_SESSION_HANDLE_INVALID, "CKR_SESSION_HANDLE_INVALID"),
    (
        CKR_SESSION_PARALLEL_NOT_SUPPORTED,
        "CKR_SESSION_PARALLEL_NOT_SUPPORTED",
    ),
    (CKR_SLOT_ID_INVALID, "CKR_SLOT_ID_INVALID"),
    (CKR_TOKEN_NOT_PRESENT, "CKR_TOKEN_NOT_PRESENT"),
    (CKR_USER_ALREADY_LOGGED_IN, "CKR_USER_ALREADY_LOGGED_IN"),
    (CKR_USER_NOT_LOGGED_IN, "CKR_USER_NOT_LOGGED_IN"),
    (CKR_USER_TYPE_INVALID, "CKR_USER_TYPE_INVALID"),
];

fn rv_name(rv: CK_RV) -> String {
    RV_NAMES
        .iter()
        .find(|(known, _)| *known == rv)
        .map_or_else(|| format!("{rv:#x}"), |(_, name)| (*name).to_owned())
}
