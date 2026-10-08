//! La única traducción de las situaciones de identidad: a la vista de la ventana y al código de la sede (ADR-0009).

use crate::crossing::Failure;
use crate::identity::application::certificates::InstallError;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::secret::SecretOnTheReaderKeypad;
use crate::identity::ports::{PromptedError, SecretPromptError};
use crate::site::domain::protocol::SafCode;

fn token_told(situation: Situation) -> (&'static str, SafCode) {
    match situation {
        Situation::IncorrectPin => ("incorrectPin", SafCode::CannotAccessKeystore),
        Situation::PinLocked => ("pinLocked", SafCode::LockedKeystore),
        Situation::TokenAbsent => ("tokenAbsent", SafCode::CannotAccessKeystore),
        Situation::ExpiredSession => ("expiredSession", SafCode::CannotAccessKeystore),
        Situation::ModuleNotFound => ("moduleNotFound", SafCode::CannotAccessKeystore),
        Situation::CertificateNotFound => {
            ("certificateNotFound", SafCode::NoCertificatesInKeystore)
        }
        Situation::Pkcs12Unreadable => ("pkcs12Unreadable", SafCode::CannotAccessKeystore),
        Situation::IncorrectPkcs12Password => {
            ("incorrectPkcs12Password", SafCode::CannotAccessKeystore)
        }
        Situation::Pkcs12NoPrivateKey => ("pkcs12NoPrivateKey", SafCode::NoCertificatesInKeystore),
        Situation::KeyKindUnsupported => ("keyKindUnsupported", SafCode::IncompatibleKeyType),
        Situation::MechanismNotOffered => ("mechanismNotOffered", SafCode::SignatureFailed),
        Situation::KeyringUnavailable => ("noKeyring", SafCode::CannotAccessKeystore),
        Situation::KeyringPinMissing => ("keyringPinMissing", SafCode::CannotAccessKeystore),
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
        Self {
            attempts_left: attempts_left_after(error.situation()),
            ..Self::new(situation_name(error.situation()), error.detail())
        }
    }
}

/// Lo único que PKCS#11 deja saber de los intentos: ninguno con el PIN bloqueado (ADR-0047).
fn attempts_left_after(situation: Situation) -> Option<u32> {
    (situation == Situation::PinLocked).then_some(0)
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

impl From<SecretPromptError> for Failure {
    fn from(error: SecretPromptError) -> Self {
        match error {
            SecretPromptError::Cancelled => Self::new(
                "userCancelled",
                "solicitud de PIN cancelada por la persona usuaria",
            ),
            SecretPromptError::Failed(reason) => Self::new("promptFailed", reason),
        }
    }
}

impl From<KeyringError> for Failure {
    fn from(error: KeyringError) -> Self {
        let situation = match error {
            KeyringError::NoKeyring => "noKeyring",
            KeyringError::PinMissing => "keyringPinMissing",
        };
        Self::new(situation, error.to_string())
    }
}

impl From<InstallError> for Failure {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::Token(error) => error.into(),
            InstallError::Store(error) => error.into(),
            InstallError::Keyring(error) => error.into(),
        }
    }
}

/// Lo que la ventana oye de instalar un `.p12`: instalado, cancelado sin error o el fallo.
pub fn installed_unless_cancelled(
    outcome: Result<(), PromptedError<InstallError>>,
) -> Result<bool, Failure> {
    match outcome {
        Ok(()) => Ok(true),
        Err(PromptedError::Prompt(SecretPromptError::Cancelled)) => Ok(false),
        Err(PromptedError::Prompt(failed)) => Err(failed.into()),
        Err(PromptedError::Attempt(install_error)) => Err(install_error.into()),
    }
}

#[cfg(test)]
mod tests;
