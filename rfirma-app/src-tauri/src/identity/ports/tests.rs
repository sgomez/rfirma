use std::cell::RefCell;
use std::sync::Mutex;

use super::{
    prompted_until_accepted, Keyring, PromptedError, SecretName, SecretPromptError,
    SecretPromptRequest, SecretPrompter,
};
use crate::identity::domain::keyring::{generate_pin, KeyringError};
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

/// Un llavero en memoria: sin llavero, con llavero sin PIN, o con un PIN ya guardado.
struct AMemoryKeyring {
    state: RefCell<Option<Option<String>>>,
}

impl AMemoryKeyring {
    fn without_a_keyring() -> Self {
        Self {
            state: RefCell::new(None),
        }
    }

    fn without_a_pin() -> Self {
        Self {
            state: RefCell::new(Some(None)),
        }
    }

    fn with_pin(pin: &str) -> Self {
        Self {
            state: RefCell::new(Some(Some(pin.to_owned()))),
        }
    }
}

impl Keyring for AMemoryKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        match self.state.borrow().as_ref() {
            None => Err(KeyringError::NoKeyring),
            Some(None) => Err(KeyringError::PinMissing),
            Some(Some(pin)) => Ok(ProtectedSecret::from_str(pin)),
        }
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        if self.state.borrow().is_none() {
            return Err(KeyringError::NoKeyring);
        }

        let pin = generate_pin();
        *self.state.borrow_mut() = Some(Some(
            pin.as_str().expect("el PIN generado es UTF-8").to_owned(),
        ));
        Ok(pin)
    }
}

#[test]
fn get_or_create_pin_creates_the_pin_the_first_time() {
    let keyring = AMemoryKeyring::without_a_pin();

    let created = keyring.get_or_create_pin().expect("crea el PIN");

    assert_eq!(keyring.pin().expect("ya esta guardado"), created);
}

#[test]
fn get_or_create_pin_reuses_the_existing_pin() {
    let keyring = AMemoryKeyring::with_pin("ya-existente");

    let pin = keyring.get_or_create_pin().expect("lee el PIN existente");

    assert_eq!(pin, ProtectedSecret::from_str("ya-existente"));
}

#[test]
fn without_a_keyring_get_or_create_pin_fails_without_creating_anything() {
    let keyring = AMemoryKeyring::without_a_keyring();

    assert_eq!(keyring.get_or_create_pin(), Err(KeyringError::NoKeyring));
}

#[test]
fn a_keyring_with_a_lost_pin_is_told_apart_from_no_keyring_at_all() {
    let lost_pin = AMemoryKeyring::without_a_pin();
    let no_keyring = AMemoryKeyring::without_a_keyring();

    assert_eq!(lost_pin.pin(), Err(KeyringError::PinMissing));
    assert_eq!(no_keyring.pin(), Err(KeyringError::NoKeyring));
}
