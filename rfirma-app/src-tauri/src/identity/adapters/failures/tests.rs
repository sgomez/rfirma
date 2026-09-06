use super::*;

const EVERY_SITUATION: [Situation; 9] = [
    Situation::IncorrectPin,
    Situation::PinLocked,
    Situation::TokenAbsent,
    Situation::ExpiredSession,
    Situation::ModuleNotFound,
    Situation::CertificateNotFound,
    Situation::Pkcs12Unreadable,
    Situation::KeyNotRsa,
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
        SafCode::CannotFindKeystore
    );
    assert_eq!(code_of_token(Situation::PinLocked), SafCode::LockedKeystore);
    assert_eq!(
        code_of_token(Situation::KeyNotRsa),
        SafCode::IncompatibleKeyType
    );
}
