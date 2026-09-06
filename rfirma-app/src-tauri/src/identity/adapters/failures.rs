//! La única traducción de las situaciones de identidad: a la vista de la ventana y al código de la sede (ADR-0009).

use crate::commands::Failure;
use crate::identity::application::certificates::InstallError;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::SecretOnTheReaderKeypad;
use crate::site::domain::protocol::SafCode;

fn token_told(situation: Situation) -> (&'static str, SafCode) {
    match situation {
        Situation::IncorrectPin => ("incorrectPin", SafCode::CannotAccessKeystore),
        Situation::PinLocked => ("pinLocked", SafCode::LockedKeystore),
        Situation::TokenAbsent => ("tokenAbsent", SafCode::CannotFindKeystore),
        Situation::ExpiredSession => ("expiredSession", SafCode::CannotAccessKeystore),
        Situation::ModuleNotFound => ("moduleNotFound", SafCode::CannotFindKeystore),
        Situation::CertificateNotFound => {
            ("certificateNotFound", SafCode::NoCertificatesInKeystore)
        }
        Situation::Pkcs12Unreadable => ("pkcs12Unreadable", SafCode::CannotAccessKeystore),
        Situation::KeyNotRsa => ("keyNotRsa", SafCode::IncompatibleKeyType),
        Situation::Unknown => ("unknown", SafCode::CannotAccessKeystore),
    }
}

/// Nombre en camelCase de una situación del almacén o token.
pub fn situation_name(situation: Situation) -> &'static str {
    token_told(situation).0
}

/// Código de protocolo de una situación del token.
pub fn code_of_token(situation: Situation) -> SafCode {
    token_told(situation).1
}

impl From<TokenError> for Failure {
    fn from(error: TokenError) -> Self {
        Self::new(situation_name(error.situation()), error.detail())
    }
}

/// Código de protocolo cuando el secreto se teclea en el lector y no se sabe pedir.
pub fn code_of_secret_on_the_reader_keypad() -> SafCode {
    SafCode::CannotAccessKeystore
}

impl From<SecretOnTheReaderKeypad> for Failure {
    fn from(refusal: SecretOnTheReaderKeypad) -> Self {
        Self::new(refusal.situation(), refusal.to_string())
    }
}

impl From<InstallError> for Failure {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::Token(error) => error.into(),
            InstallError::Store(error) => error.into(),
        }
    }
}

#[cfg(test)]
mod tests;
