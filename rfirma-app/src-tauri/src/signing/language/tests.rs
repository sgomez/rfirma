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
