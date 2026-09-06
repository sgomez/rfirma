use super::{situation_name, Failure};
use crate::pkcs11::Situation;

#[test]
fn every_token_situation_has_a_camel_case_name_for_the_catalogue() {
    let all = [
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
    for situation in all {
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
    let failure: Failure = crate::pkcs11::TokenError::new(
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
