use super::{localize, MockSecretPrompter, PreconfiguredSecretPrompter};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;
use crate::signing::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};

#[test]
fn localizes_into_all_five_official_languages() {
    for lang in Language::ALL {
        // Con subject y sin attempts_left
        let req1 = SecretPromptRequest {
            token_label: "DNIe".to_string(),
            subject: Some("JUAN PEREZ".to_string()),
            language: lang,
            incorrect_pin: false,
            attempts_left: None,
        };
        let i18n1 = localize(&req1);
        assert!(!i18n1.title.is_empty());
        assert!(i18n1.prompt.contains("JUAN PEREZ"));
        assert!(!i18n1.incorrect_pin.is_empty());
        assert!(!i18n1.accept.is_empty());
        assert!(!i18n1.cancel.is_empty());

        // Sin subject y con attempts_left
        let req2 = SecretPromptRequest {
            token_label: "DNIe".to_string(),
            subject: None,
            language: lang,
            incorrect_pin: true,
            attempts_left: Some(2),
        };
        let i18n2 = localize(&req2);
        assert!(i18n2.prompt.contains("DNIe"));
        assert!(i18n2.incorrect_pin.contains("2"));
    }
}

#[test]
fn localizes_incorrect_pin_with_attempts_remaining() {
    let req = SecretPromptRequest {
        token_label: "FNMT".to_string(),
        subject: None,
        language: Language::English,
        incorrect_pin: true,
        attempts_left: Some(2),
    };
    let i18n = localize(&req);
    assert_eq!(i18n.incorrect_pin, "Incorrect PIN. 2 attempts remaining.");
    assert_eq!(i18n.title, "Enter PIN");
    assert_eq!(i18n.accept, "OK");
    assert_eq!(i18n.cancel, "Cancel");
}

#[test]
fn preconfigured_prompter_yields_secret() {
    let prompter = PreconfiguredSecretPrompter::new("9876");
    let req = SecretPromptRequest {
        token_label: "test".to_string(),
        subject: None,
        language: Language::Spanish,
        incorrect_pin: false,
        attempts_left: None,
    };
    let secret = prompter
        .prompt_secret(&req)
        .expect("deberia devolver secreto");
    assert_eq!(secret.as_str(), Ok("9876"));
}

#[test]
fn preconfigured_prompter_cancelling_yields_cancelled() {
    let prompter = PreconfiguredSecretPrompter::cancelling();
    let req = SecretPromptRequest {
        token_label: "test".to_string(),
        subject: None,
        language: Language::Spanish,
        incorrect_pin: false,
        attempts_left: None,
    };
    let error = prompter
        .prompt_secret(&req)
        .expect_err("deberia ser cancelado");
    assert_eq!(error, SecretPromptError::Cancelled);
}

#[test]
fn mock_prompter_records_requests_and_serves_responses() {
    let mock = MockSecretPrompter::with_responses(vec![
        Err(SecretPromptError::Cancelled),
        Ok(ProtectedSecret::from_str("correct_pin")),
    ]);

    let req1 = SecretPromptRequest {
        token_label: "slot1".to_string(),
        subject: Some("User 1".to_string()),
        language: Language::Galician,
        incorrect_pin: false,
        attempts_left: None,
    };
    let res1 = mock.prompt_secret(&req1);
    assert_eq!(res1, Err(SecretPromptError::Cancelled));

    let req2 = SecretPromptRequest {
        token_label: "slot1".to_string(),
        subject: Some("User 1".to_string()),
        language: Language::Galician,
        incorrect_pin: true,
        attempts_left: Some(2),
    };
    let res2 = mock.prompt_secret(&req2).expect("debe ser Ok");
    assert_eq!(res2.as_str(), Ok("correct_pin"));

    let recorded = mock.recorded_requests();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].token_label, "slot1");
    assert!(!recorded[0].incorrect_pin);
    assert!(recorded[1].incorrect_pin);
}
