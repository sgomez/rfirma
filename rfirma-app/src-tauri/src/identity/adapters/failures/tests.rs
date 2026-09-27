use super::*;

const EVERY_SITUATION: [Situation; 12] = [
    Situation::IncorrectPin,
    Situation::PinLocked,
    Situation::TokenAbsent,
    Situation::ExpiredSession,
    Situation::ModuleNotFound,
    Situation::CertificateNotFound,
    Situation::Pkcs12Unreadable,
    Situation::IncorrectPkcs12Password,
    Situation::Pkcs12NoPrivateKey,
    Situation::KeyKindUnsupported,
    Situation::MechanismNotOffered,
    Situation::Unknown,
];

#[test]
fn every_token_situation_has_a_camel_case_name_for_the_catalogue() {
    for situation in EVERY_SITUATION {
        let name = situation_name(situation);
        assert!(!name.is_empty());
        assert!(
            !name.contains('_') && name.chars().next().is_some_and(char::is_lowercase),
            "«{name}» no está en camelCase"
        );
    }
}

#[test]
fn a_failure_keeps_the_raw_detail_of_the_token() {
    let failure: Failure = TokenError::new(
        Situation::CertificateNotFound,
        "el token no tiene ninguna clave privada etiquetada X",
    )
    .into();

    assert_eq!(failure.situation, "certificateNotFound");
    assert_eq!(
        failure.detail,
        "el token no tiene ninguna clave privada etiquetada X"
    );
}

#[test]
fn the_window_and_the_site_hear_about_a_missing_token_from_the_same_line() {
    assert_eq!(situation_name(Situation::TokenAbsent), "tokenAbsent");
    assert_eq!(
        code_of_token(Situation::TokenAbsent),
        SafCode::CannotAccessKeystore
    );
    assert_eq!(code_of_token(Situation::PinLocked), SafCode::LockedKeystore);
    assert_eq!(
        code_of_token(Situation::KeyKindUnsupported),
        SafCode::IncompatibleKeyType
    );
}

#[test]
fn the_three_ways_a_p12_can_fail_have_three_different_names() {
    assert_eq!(
        situation_name(Situation::IncorrectPkcs12Password),
        "incorrectPkcs12Password"
    );
    assert_eq!(
        situation_name(Situation::Pkcs12Unreadable),
        "pkcs12Unreadable"
    );
    assert_eq!(
        situation_name(Situation::Pkcs12NoPrivateKey),
        "pkcs12NoPrivateKey"
    );
    assert_eq!(
        code_of_token(Situation::Pkcs12NoPrivateKey),
        SafCode::NoCertificatesInKeystore
    );
}

#[test]
fn a_mechanism_the_token_does_not_offer_fails_the_signature_as_in_the_original() {
    assert_eq!(
        situation_name(Situation::MechanismNotOffered),
        "mechanismNotOffered"
    );
    assert_eq!(
        code_of_token(Situation::MechanismNotOffered),
        SafCode::SignatureFailed
    );
}

#[test]
fn a_missing_keyring_and_a_missing_pin_have_two_different_names() {
    assert_eq!(situation_name(Situation::KeyringUnavailable), "noKeyring");
    assert_eq!(
        situation_name(Situation::KeyringPinMissing),
        "keyringPinMissing"
    );
    assert_eq!(
        code_of_token(Situation::KeyringUnavailable),
        SafCode::CannotAccessKeystore
    );
}

#[test]
fn an_installed_p12_answers_true() {
    assert_eq!(installed_unless_cancelled(Ok(())), Ok(true));
}

#[test]
fn a_cancelled_p12_password_answers_false_without_a_failure() {
    let outcome = Err(PromptedError::Prompt(SecretPromptError::Cancelled));

    assert_eq!(installed_unless_cancelled(outcome), Ok(false));
}

#[test]
fn a_password_dialog_that_fails_crosses_as_a_failed_prompt() {
    let outcome = Err(PromptedError::Prompt(SecretPromptError::Failed(
        "sin pantalla".to_string(),
    )));

    let failure = installed_unless_cancelled(outcome).unwrap_err();

    assert_eq!(failure.situation, "promptFailed");
}

#[test]
fn a_p12_the_token_refuses_crosses_with_the_situation_of_the_token() {
    let outcome = Err(PromptedError::Attempt(InstallError::Token(
        TokenError::new(Situation::Pkcs12NoPrivateKey, "sin clave"),
    )));

    let failure = installed_unless_cancelled(outcome).unwrap_err();

    assert_eq!(failure.situation, "pkcs12NoPrivateKey");
}
