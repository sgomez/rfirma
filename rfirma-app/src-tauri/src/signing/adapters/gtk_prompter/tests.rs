use super::{localize, MockSecretPrompter, PreconfiguredSecretPrompter};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;
use crate::signing::ports::{
    PromptedHolder, SecretName, SecretPromptError, SecretPromptRequest, SecretPrompter,
};

fn a_request(holder: Option<PromptedHolder>, language: Language) -> SecretPromptRequest {
    SecretPromptRequest {
        secret: SecretName::Pin,
        holder,
        language,
        incorrect_secret: false,
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
        for secret in [SecretName::Pin, SecretName::Password] {
            let mut request = a_request(Some(juan_perez()), lang);
            request.secret = secret;

            let i18n = localize(&request);

            assert!(!i18n.title.is_empty());
            assert!(!i18n.incorrect_secret.is_empty());
            assert!(!i18n.accept.is_empty());
            assert!(!i18n.cancel.is_empty());
            assert_eq!(i18n.holder_name.as_deref(), Some("JUAN PEREZ"));
            assert_eq!(i18n.id_number.as_deref(), Some("IDCES-12345678Z"));
        }
    }
}

#[test]
fn a_module_is_asked_for_a_pin_and_a_file_store_for_a_password() {
    let mut request = a_request(None, Language::Spanish);

    request.secret = SecretName::Pin;
    let module = localize(&request);
    assert_eq!(module.title, "Introduce el PIN");
    assert_eq!(
        module.incorrect_secret,
        "PIN incorrecto. Vuelve a intentarlo."
    );

    request.secret = SecretName::Password;
    let file = localize(&request);
    assert_eq!(file.title, "Introduce la contraseña");
    assert_eq!(
        file.incorrect_secret,
        "Contraseña incorrecta. Vuelve a intentarlo."
    );
}

#[test]
fn no_text_of_the_dialog_ever_counts_attempts() {
    for lang in Language::ALL {
        for secret in [SecretName::Pin, SecretName::Password] {
            let mut request = a_request(Some(juan_perez()), lang);
            request.secret = secret;
            request.incorrect_secret = true;

            let i18n = localize(&request);

            assert!(
                !i18n.incorrect_secret.contains(|c: char| c.is_ascii_digit()),
                "PKCS#11 no cuenta intentos y el dialogo no puede prometerlos: {}",
                i18n.incorrect_secret
            );
        }
    }
}

#[test]
fn shows_the_holder_and_never_the_label_of_the_store() {
    let i18n = localize(&a_request(Some(juan_perez()), Language::Spanish));

    assert_eq!(i18n.holder_name.as_deref(), Some("JUAN PEREZ"));
    assert_eq!(i18n.id_number.as_deref(), Some("IDCES-12345678Z"));
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
    second.incorrect_secret = true;
    let secret = mock.prompt_secret(&second).expect("debe ser Ok");
    assert_eq!(secret.as_str(), Ok("correct_pin"));

    let recorded = mock.recorded_requests();
    assert_eq!(recorded.len(), 2);
    assert!(!recorded[0].incorrect_secret);
    assert!(recorded[1].incorrect_secret);
}
