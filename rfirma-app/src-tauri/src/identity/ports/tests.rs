use std::sync::Mutex;

use super::{
    prompted_until_accepted, PromptedError, SecretName, SecretPromptError, SecretPromptRequest,
    SecretPrompter,
};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;

/// Un prompter que devuelve, en orden, los secretos o fallos programados.
struct AScriptedPrompter {
    responses: Mutex<Vec<Result<ProtectedSecret, SecretPromptError>>>,
    recorded: Mutex<Vec<SecretPromptRequest>>,
}

impl AScriptedPrompter {
    fn with(responses: Vec<Result<ProtectedSecret, SecretPromptError>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            recorded: Mutex::new(Vec::new()),
        }
    }
}

impl SecretPrompter for AScriptedPrompter {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        self.recorded.lock().unwrap().push(request.clone());
        self.responses.lock().unwrap().remove(0)
    }
}

fn a_request() -> SecretPromptRequest {
    SecretPromptRequest {
        secret: SecretName::Pin,
        holder: None,
        language: Language::Spanish,
        incorrect_secret: false,
    }
}

#[test]
fn accepts_the_secret_on_the_first_try() {
    let prompter = AScriptedPrompter::with(vec![Ok(ProtectedSecret::from_str("1234"))]);

    let (secret, done) = prompted_until_accepted(
        &prompter,
        a_request(),
        |secret| -> Result<(), &'static str> {
            assert_eq!(secret.expose_secret(), Ok("1234"));
            Ok(())
        },
        |_| false,
    )
    .expect("el primer intento ya vale");

    assert_eq!(secret.expose_secret(), Ok("1234"));
    assert_eq!(done, ());
    assert_eq!(prompter.recorded.lock().unwrap().len(), 1);
}

#[test]
fn retries_only_when_the_attempt_says_the_secret_was_rejected() {
    let prompter = AScriptedPrompter::with(vec![
        Ok(ProtectedSecret::from_str("wrong")),
        Ok(ProtectedSecret::from_str("right")),
    ]);

    let (secret, ()) = prompted_until_accepted(
        &prompter,
        a_request(),
        |secret| {
            if secret.expose_secret() == Ok("right") {
                Ok(())
            } else {
                Err("rejected")
            }
        },
        |error| *error == "rejected",
    )
    .expect("el segundo intento acierta");

    assert_eq!(secret.expose_secret(), Ok("right"));
    let recorded = prompter.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 2, "se pidio dos veces");
    assert!(!recorded[0].incorrect_secret);
    assert!(
        recorded[1].incorrect_secret,
        "la segunda vez avisa del error"
    );
}

#[test]
fn stops_without_retrying_when_the_attempt_is_not_a_rejection() {
    let prompter = AScriptedPrompter::with(vec![Ok(ProtectedSecret::from_str("1234"))]);

    let error = prompted_until_accepted(
        &prompter,
        a_request(),
        |_| -> Result<(), &'static str> { Err("unrelated failure") },
        |error| *error == "rejected",
    )
    .expect_err("un rechazo que no es de secreto no se reintenta");

    assert!(matches!(error, PromptedError::Attempt("unrelated failure")));
    assert_eq!(
        prompter.recorded.lock().unwrap().len(),
        1,
        "no hay segundo intento"
    );
}

#[test]
fn a_cancelled_prompt_stops_the_loop_without_attempting() {
    let prompter = AScriptedPrompter::with(vec![Err(SecretPromptError::Cancelled)]);

    let error = prompted_until_accepted(
        &prompter,
        a_request(),
        |_| -> Result<(), &'static str> { unreachable!("cancelar no llega a intentarlo") },
        |_| true,
    )
    .expect_err("cancelar el dialogo aborta");

    assert!(matches!(
        error,
        PromptedError::Prompt(SecretPromptError::Cancelled)
    ));
}
