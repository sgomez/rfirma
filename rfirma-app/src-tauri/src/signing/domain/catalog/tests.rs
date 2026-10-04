use super::po_file::{entries_of, is_complete};
use super::*;

const SPANISH_PO: &str = include_str!("../../../../../po/es.po");
const ENGLISH_PO: &str = include_str!("../../../../../po/en.po");

fn msgstr_of(po: &str, key: &str) -> String {
    entries_of(po)
        .into_iter()
        .find(|(entry, _)| entry == key)
        .map(|(_, text)| text)
        .expect("la clave está en el .po")
}

#[test]
fn a_key_reads_the_msgstr_of_its_language() {
    assert_eq!(
        translated(Language::English, "signatureReason.damaged", &[]),
        msgstr_of(ENGLISH_PO, "signatureReason.damaged")
    );
    assert_eq!(
        translated(Language::Spanish, "signatureReason.damaged", &[]),
        msgstr_of(SPANISH_PO, "signatureReason.damaged")
    );
}

#[test]
fn a_language_that_is_not_published_falls_back_to_spanish() {
    let published: &[(&str, &[(&str, &str)])] = &[("es", &[("greeting", "Hola")])];

    assert_eq!(
        text_in(published, Language::Basque, "greeting"),
        Some("Hola")
    );
}

#[test]
fn an_empty_text_falls_back_to_spanish() {
    let published: &[(&str, &[(&str, &str)])] =
        &[("es", &[("greeting", "Hola")]), ("en", &[("greeting", "")])];

    assert_eq!(
        text_in(published, Language::English, "greeting"),
        Some("Hola")
    );
}

#[test]
fn an_unknown_key_is_written_as_the_key_like_i18next() {
    assert_eq!(
        translated(Language::Spanish, "no.such.key", &[]),
        "no.such.key"
    );
}

#[test]
fn the_variables_are_replaced_like_i18next() {
    assert_eq!(
        interpolated(
            "El certificado de {{holder}} caducó el {{ date }}",
            &[("holder", "ACME SL"), ("date", "2026-01-02")]
        ),
        "El certificado de ACME SL caducó el 2026-01-02"
    );
}

#[test]
fn a_variable_without_a_value_is_left_as_written_like_i18next() {
    assert_eq!(
        interpolated("{{name}} no admitía más firmas", &[]),
        "{{name}} no admitía más firmas"
    );
}

#[test]
fn the_po_entries_join_continued_lines_and_unescape() {
    let po = "msgid \"\"\nmsgstr \"\"\n\"Language: es\\n\"\n\nmsgid \"a.key\"\nmsgstr \"\"\n\"Uno \\\"dos\\\" \"\n\"tres\"\n";

    assert_eq!(
        entries_of(po),
        vec![("a.key".to_owned(), "Uno \"dos\" tres".to_owned())]
    );
}

#[test]
fn a_plural_entry_gives_one_key_per_spanish_plural_form() {
    let po = "msgid \"pages\"\nmsgid_plural \"pages\"\nmsgstr[0] \"1 página\"\nmsgstr[1] \"{{count}} de páginas\"\nmsgstr[2] \"{{count}} páginas\"\n";

    assert_eq!(
        entries_of(po),
        vec![
            ("pages_one".to_owned(), "1 página".to_owned()),
            ("pages_many".to_owned(), "{{count}} de páginas".to_owned()),
            ("pages_other".to_owned(), "{{count}} páginas".to_owned()),
        ]
    );
}

#[test]
fn a_fuzzy_entry_counts_as_untranslated() {
    let po = "#, fuzzy\nmsgid \"a.key\"\nmsgstr \"Casi\"\n\nmsgid \"b.key\"\nmsgstr \"Sí\"\n";

    let entries = entries_of(po);

    assert_eq!(entries[0], ("a.key".to_owned(), String::new()));
    assert!(!is_complete(&entries));
}

#[test]
fn obsolete_entries_are_not_read() {
    let po = "msgid \"a.key\"\nmsgstr \"Sí\"\n\n#~ msgid \"old.key\"\n#~ msgstr \"Viejo\"\n";

    assert_eq!(entries_of(po), vec![("a.key".to_owned(), "Sí".to_owned())]);
}

#[test]
fn the_catalog_holds_spanish_and_only_the_complete_languages() {
    let tags: Vec<&str> = PUBLISHED.iter().map(|(tag, _)| *tag).collect();

    assert!(tags.contains(&"es"), "{tags:?}");
    for language in Language::ALL {
        let po = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../po")
                .join(format!("{}.po", language.tag())),
        )
        .expect("el .po está versionado");
        assert_eq!(
            tags.contains(&language.tag()),
            is_complete(&entries_of(&po)),
            "{}",
            language.tag()
        );
    }
}
