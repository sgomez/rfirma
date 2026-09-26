//! Compone el ciclo de firma con el diálogo interactivo del secreto, cuyo puerto es de `identity` (ADR-0001, ADR-0014).

use crate::crossing::Failure;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::holder::prompted_holder_of;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{SecretName, StoreSecret};
use crate::identity::ports::{
    prompted_until_accepted, PromptedError, SecretPromptRequest, SecretPrompter,
};
use crate::signing::application::cycle::CycleError;
use crate::signing::application::session::{self, CycleFailure, SigningSession};
use crate::signing::domain::Language;
use crate::signing::ports::Signer;

fn secret_was_rejected(failure: &CycleFailure) -> bool {
    matches!(
        failure,
        CycleFailure::Cycle(CycleError::Token(error)) if error.situation() == Situation::IncorrectPin
    )
}

fn cycle_prompt_failure(error: PromptedError<CycleFailure>) -> Failure {
    match error {
        PromptedError::Prompt(prompt) => Failure::from(prompt),
        PromptedError::Attempt(failure) => Failure::from(failure),
    }
}

/// Fase de firma en el token PKCS#11 solicitando el secreto mediante el diálogo interactivo,
/// con reintento si el token dice que el PIN era incorrecto (ADR-0001, ADR-0014).
pub fn sign_on_token_with_prompter(
    signer: &dyn Signer,
    session: &SigningSession,
    prompter: &dyn SecretPrompter,
    language: Language,
) -> Result<(), Failure> {
    let (secret, holder) = session::secret_prompt_context(session)?;
    let request = SecretPromptRequest {
        secret,
        holder,
        language,
        incorrect_secret: false,
    };
    prompted_until_accepted(
        prompter,
        request,
        |typed| session::sign_on_token(signer, session, typed),
        secret_was_rejected,
    )
    .map(|_| ())
    .map_err(cycle_prompt_failure)
}

/// Firma el ciclo abierto con el secreto tecleado, o pidiéndolo al diálogo si llega vacío.
pub fn signed_on_the_token(
    signer: &dyn Signer,
    session: &SigningSession,
    prompter: &dyn SecretPrompter,
    language: Language,
    secret: &ProtectedSecret,
) -> Result<(), Failure> {
    if secret.is_empty() {
        return sign_on_token_with_prompter(signer, session, prompter, language);
    }
    Ok(session::sign_on_token(signer, session, secret)?)
}

fn token_secret_rejected(error: &TokenError) -> bool {
    error.situation() == Situation::IncorrectPin
}

/// El secreto del lote: el tecleado, el que el token acepta tras el diálogo, o vacío si no lo pide.
pub fn secret_for_the_batch(
    signer: &dyn Signer,
    certificate: &TokenCertificate,
    prompter: &dyn SecretPrompter,
    language: Language,
    typed: &ProtectedSecret,
) -> Result<ProtectedSecret, Failure> {
    if !typed.is_empty() {
        return Ok(ProtectedSecret::new(typed.as_bytes()));
    }
    let mode = signer.secret_of(certificate.reference())?;
    if mode != StoreSecret::TypedOnScreen {
        return Ok(ProtectedSecret::new(b""));
    }
    let request = SecretPromptRequest {
        secret: SecretName::of(certificate.reference().store().class()),
        holder: prompted_holder_of(certificate.der()),
        language,
        incorrect_secret: false,
    };
    let (secret, ()) = prompted_until_accepted(
        prompter,
        request,
        |secret| signer.accepts_the_secret(certificate.reference(), secret),
        token_secret_rejected,
    )
    .map_err(|error| match error {
        PromptedError::Prompt(prompt) => Failure::from(prompt),
        PromptedError::Attempt(token_error) => Failure::from(token_error),
    })?;
    Ok(secret)
}

#[cfg(test)]
mod tests;
