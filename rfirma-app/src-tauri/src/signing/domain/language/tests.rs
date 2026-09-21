use super::Language;
use std::collections::HashSet;

#[test]
fn covers_the_five_languages_of_the_adr() {
    assert_eq!(Language::ALL.len(), 5);
    assert_eq!(
        Language::ALL.map(Language::tag),
        ["es", "ca", "eu", "gl", "en"],
        "los cinco idiomas no coinciden con los esperados"
    );
}

#[test]
fn gives_every_language_its_own_tag() {
    let tags: HashSet<&str> = Language::ALL.iter().map(|l| l.tag()).collect();
    assert_eq!(tags.len(), Language::ALL.len());
}

#[test]
fn is_persisted_by_the_very_tag_it_reports() {
    for language in Language::ALL {
        assert_eq!(
            serde_json::to_value(language).expect("deberia serializarse"),
            serde_json::json!(language.tag()),
            "el rename de serde y tag() se han separado en {language:?}"
        );
        assert_eq!(
            serde_json::from_value::<Language>(serde_json::json!(language.tag()))
                .expect("deberia leerse"),
            language,
        );
    }
}

#[test]
fn recognises_the_language_of_a_posix_locale() {
    assert_eq!(Language::of_locale("ca_ES.UTF-8"), Some(Language::Catalan));
    assert_eq!(Language::of_locale("gl_ES@euro"), Some(Language::Galician));
    assert_eq!(Language::of_locale("eu"), Some(Language::Basque));
}

#[test]
fn recognises_the_language_of_a_bcp47_tag() {
    assert_eq!(Language::of_locale("en-GB"), Some(Language::English));
    assert_eq!(Language::of_locale("ES-es"), Some(Language::Spanish));
    assert_eq!(
        Language::of_locale("ca-ES-valencia"),
        Some(Language::Catalan)
    );
}

#[test]
fn does_not_recognise_an_unsupported_or_neutral_locale() {
    assert_eq!(Language::of_locale("fr_FR.UTF-8"), None);
    assert_eq!(Language::of_locale("C.UTF-8"), None);
    assert_eq!(Language::of_locale(""), None);
}

#[test]
fn takes_the_first_supported_language_among_the_preferred_ones() {
    assert_eq!(
        Language::first_of(["fr-FR", "gl-ES", "en-US"]),
        Language::Galician
    );
}

#[test]
fn falls_back_to_spanish_when_no_preferred_language_is_supported() {
    assert_eq!(Language::first_of(["fr-FR", "de-DE"]), Language::Spanish);
    assert_eq!(Language::first_of(Vec::<String>::new()), Language::Spanish);
}
