//! El PIN del DNIe bajo el bloqueo de versión de rdnie: los intentos que vio la persona y los que guarda la tarjeta (ADR-0047).

use rdnie::{Session, Transport};

use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;

/// `pin_operation` bajo el bloqueo de versión de rdnie; en una sesión que no lo conoce, con los intentos de `shown` si la tarjeta aún los tiene.
pub(super) fn under_the_retries_lock<T: Transport, R>(
    session: &mut Session<T>,
    shown: Option<u8>,
    mut pin_operation: impl FnMut(&mut Session<T>) -> Result<R, rdnie::Error>,
) -> Result<Result<R, TokenError>, rdnie::Error> {
    let done = match pin_operation(session) {
        Err(rdnie::Error::PinRetriesNotRead) => {
            let left = session.pin_retries_left()?;
            if let Err(changed) = unchanged_since_shown(left, shown) {
                return Ok(Err(changed));
            }
            pin_operation(session)
        }
        done => done,
    };
    match done {
        Err(rdnie::Error::PinRetriesChanged) => {
            let left = retries_left_or_blocked(session)?;
            Ok(Err(attempts_changed(shown, left)))
        }
        done => done.map(Ok),
    }
}

pub(super) fn retries_left_or_blocked<T: Transport>(
    session: &mut Session<T>,
) -> Result<u8, rdnie::Error> {
    match session.pin_retries_left() {
        Err(rdnie::Error::PinBlocked) => Ok(0),
        read => read,
    }
}

pub(super) fn typed_pin(secret: &ProtectedSecret) -> Result<&str, TokenError> {
    secret
        .expose_secret()
        .map_err(|_| TokenError::new(Situation::IncorrectPin, "el PIN no es texto"))
}

fn unchanged_since_shown(left: u8, shown: Option<u8>) -> Result<(), TokenError> {
    match shown {
        Some(seen) if seen != left => Err(attempts_changed(shown, left)),
        _ => Ok(()),
    }
}

fn attempts_changed(shown: Option<u8>, left: u8) -> TokenError {
    let since = shown.map(|seen| format!("de {seen} ")).unwrap_or_default();
    TokenError::new(
        Situation::Unknown,
        format!("los intentos han pasado {since}a {left} tras leerlos: el PIN no se envia"),
    )
}

pub(super) fn has_a_length_the_dnie_admits(secret: &ProtectedSecret) -> bool {
    secret
        .expose_secret()
        .is_ok_and(|pin| (8..=16).contains(&pin.encode_utf16().count()))
}
