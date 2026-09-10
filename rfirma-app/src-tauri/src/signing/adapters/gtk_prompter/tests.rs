use super::{localize, MockSecretPrompter, PreconfiguredSecretPrompter};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;
use crate::signing::ports::{
    PromptedHolder, SecretPromptError, SecretPromptRequest, SecretPrompter,
};

fn a_request(holder: Option<PromptedHolder>, language: Language) -> SecretPromptRequest {
    SecretPromptRequest {
        token_label: "DNIe".to_string(),
        holder,
        language,
        incorrect_pin: false,
        attempts_left: None,
    }
}

fn juan_perez() -> PromptedHolder {
    PromptedHolder {
        name: "JUAN PEREZ".to_string(),
        id_number: "IDCES-12345678Z".to_string(),
    }
}

#[test]
fn localizes_into_all_five_official_languages() {
    for lang in Language::ALL {
        let with_holder = localize(&a_request(Some(juan_perez()), lang));
        assert!(!with_holder.title.is_empty());
        assert_eq!(with_holder.holder_name.as_deref(), Some("JUAN PEREZ"));
        assert_eq!(with_holder.id_number.as_deref(), Some("IDCES-12345678Z"));
        assert!(!with_holder.incorrect_pin.is_empty());
        assert!(!with_holder.accept.is_empty());
        assert!(!with_holder.cancel.is_empty());

        let mut without_holder = a_request(None, lang);
        without_holder.incorrect_pin = true;
        without_holder.attempts_left = Some(2);
        let anonymous = localize(&without_holder);
        assert_eq!(anonymous.holder_name, None);
        assert_eq!(anonymous.id_number, None);
        assert!(anonymous.incorrect_pin.contains('2'));
    }
}

#[test]
fn shows_the_holder_and_never_the_label_of_the_store() {
    let i18n = localize(&a_request(Some(juan_perez()), Language::Spanish));

    assert_eq!(i18n.holder_name.as_deref(), Some("JUAN PEREZ"));
    assert!(!i18n
        .id_number
        .as_deref()
        .unwrap_or_default()
        .contains("DNIe"));
}

#[test]
fn omits_the_identifier_when_the_certificate_carries_none() {
    let nameless_number = PromptedHolder {
        name: "ENTIDAD SL".to_string(),
        id_number: String::new(),
    };

    let i18n = localize(&a_request(Some(nameless_number), Language::Spanish));

    assert_eq!(i18n.holder_name.as_deref(), Some("ENTIDAD SL"));
    assert_eq!(i18n.id_number, None);
}

#[test]
fn localizes_incorrect_pin_with_attempts_remaining() {
    let mut request = a_request(None, Language::English);
    request.incorrect_pin = true;
    request.attempts_left = Some(2);

    let i18n = localize(&request);

    assert_eq!(i18n.incorrect_pin, "Incorrect PIN. 2 attempts remaining.");
    assert_eq!(i18n.title, "Enter PIN");
    assert_eq!(i18n.accept, "OK");
    assert_eq!(i18n.cancel, "Cancel");
}

#[test]
fn preconfigured_prompter_yields_secret() {
    let prompter = PreconfiguredSecretPrompter::new("9876");

    let secret = prompter
        .prompt_secret(&a_request(None, Language::Spanish))
        .expect("deberia devolver secreto");

    assert_eq!(secret.as_str(), Ok("9876"));
}

#[test]
fn preconfigured_prompter_cancelling_yields_cancelled() {
    let prompter = PreconfiguredSecretPrompter::cancelling();

    let error = prompter
        .prompt_secret(&a_request(None, Language::Spanish))
        .expect_err("deberia ser cancelado");

    assert_eq!(error, SecretPromptError::Cancelled);
}

#[test]
fn mock_prompter_records_requests_and_serves_responses() {
    let mock = MockSecretPrompter::with_responses(vec![
        Err(SecretPromptError::Cancelled),
        Ok(ProtectedSecret::from_str("correct_pin")),
    ]);

    let first = a_request(Some(juan_perez()), Language::Galician);
    assert_eq!(
        mock.prompt_secret(&first),
        Err(SecretPromptError::Cancelled)
    );

    let mut second = a_request(Some(juan_perez()), Language::Galician);
    second.incorrect_pin = true;
    second.attempts_left = Some(2);
    let secret = mock.prompt_secret(&second).expect("debe ser Ok");
    assert_eq!(secret.as_str(), Ok("correct_pin"));

    let recorded = mock.recorded_requests();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].token_label, "DNIe");
    assert!(!recorded[0].incorrect_pin);
    assert!(recorded[1].incorrect_pin);
}
