use super::{localize, MockSecretPrompter, PreconfiguredSecretPrompter};
use crate::identity::domain::holder::PromptedHolder;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, SecretName};
use crate::identity::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};
use crate::signing::domain::Language;

fn a_request(holder: Option<PromptedHolder>, language: Language) -> SecretPromptRequest {
    SecretPromptRequest {
        secret: SecretName::Pin,
        holder,
        language,
        incorrect_secret: false,
        pin_warning: PinWarning::Quiet,
        origin_window: None,
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
        for secret in [
            SecretName::Pin,
            SecretName::Password,
            SecretName::DocumentPassword,
            SecretName::Pkcs12Password("certificado.p12".to_string()),
        ] {
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
fn a_locked_pdf_is_asked_for_its_own_password() {
    let mut request = a_request(None, Language::Spanish);
    request.secret = SecretName::DocumentPassword;

    let document = localize(&request);

    assert_eq!(document.title, "Introduce la contraseña del PDF");
    assert_eq!(
        document.incorrect_secret,
        "Contraseña del PDF incorrecta. Vuelve a intentarlo."
    );
}

#[test]
fn installing_a_pkcs12_names_the_file_and_never_says_store() {
    let mut request = a_request(None, Language::Spanish);
    request.secret = SecretName::Pkcs12Password("certificado.p12".to_string());

    let dialog = localize(&request);

    assert!(dialog.title.contains("certificado.p12"), "{}", dialog.title);
    assert!(!dialog.title.to_lowercase().contains("almacen"));
    assert!(!dialog.title.to_lowercase().contains("almacén"));
}

#[test]
fn no_text_of_the_dialog_ever_counts_attempts() {
    for lang in Language::ALL {
        for secret in [
            SecretName::Pin,
            SecretName::Password,
            SecretName::DocumentPassword,
            SecretName::Pkcs12Password("certificado.p12".to_string()),
        ] {
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
fn a_card_on_its_final_try_is_warned_before_typing_and_one_with_failed_tries_more_softly() {
    let mut request = a_request(None, Language::Spanish);

    request.pin_warning = PinWarning::FinalTry;
    assert_eq!(
        localize(&request).pin_warning.as_deref(),
        Some("Último intento: si el PIN no es correcto, la tarjeta se bloqueará.")
    );

    request.pin_warning = PinWarning::CountLow;
    assert_eq!(
        localize(&request).pin_warning.as_deref(),
        Some("Ya ha habido algún intento fallido con esta tarjeta.")
    );

    request.pin_warning = PinWarning::Quiet;
    assert_eq!(localize(&request).pin_warning, None);
}

#[test]
fn the_pin_warnings_are_in_all_five_languages_and_count_nothing() {
    for lang in Language::ALL {
        let mut request = a_request(None, lang);
        request.pin_warning = PinWarning::FinalTry;
        let last = localize(&request)
            .pin_warning
            .expect("aviso de último intento");
        request.pin_warning = PinWarning::CountLow;
        let soft = localize(&request).pin_warning.expect("aviso suave");

        assert_ne!(last, soft, "{lang:?}");
        for warning in [&last, &soft] {
            assert!(
                !warning.contains(|c: char| c.is_ascii_digit()),
                "PKCS#11 no cuenta intentos: {warning}"
            );
        }
    }
}

#[test]
fn a_password_is_never_asked_with_a_card_warning() {
    let mut request = a_request(None, Language::Spanish);
    request.secret = SecretName::Password;
    request.pin_warning = PinWarning::FinalTry;

    assert_eq!(localize(&request).pin_warning, None);
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
