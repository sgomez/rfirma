//! El catálogo de cadenas de la ventana dentro del binario, leído de los `.po` al compilar (ADR-0009); no decide el idioma.

use super::language::Language;

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));

type Published<'a> = &'a [(&'a str, &'a [(&'a str, &'a str)])];

/// El texto de una clave en un idioma, con sus `{{variables}}` resueltas como en i18next.
pub fn translated(language: Language, key: &str, values: &[(&str, &str)]) -> String {
    interpolated(text_in(PUBLISHED, language, key).unwrap_or(key), values)
}

/// El texto de una clave con plural para `count`, en la forma que elige `Intl.PluralRules` del idioma.
pub fn counted(language: Language, key: &str, count: usize) -> String {
    let form = plural_form(language, count);
    translated(
        language,
        &format!("{key}_{form}"),
        &[("count", &count.to_string())],
    )
}

fn plural_form(language: Language, count: usize) -> &'static str {
    let has_many = matches!(language, Language::Spanish | Language::Catalan);
    match count {
        1 => "one",
        count if has_many && count != 0 && count % 1_000_000 == 0 => "many",
        _ => "other",
    }
}

fn text_in<'a>(published: Published<'a>, language: Language, key: &str) -> Option<&'a str> {
    let in_tag = |tag: &str| {
        published
            .iter()
            .find(|(published_tag, _)| *published_tag == tag)
            .and_then(|(_, entries)| entries.iter().find(|(entry, _)| *entry == key))
            .map(|(_, text)| *text)
            .filter(|text| !text.is_empty())
    };
    in_tag(language.tag()).or_else(|| in_tag(Language::Spanish.tag()))
}

fn interpolated(template: &str, values: &[(&str, &str)]) -> String {
    let mut text = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let Some(length) = rest[start + 2..].find("}}") else {
            break;
        };
        let placeholder = &rest[start..start + 2 + length + 2];
        let name = rest[start + 2..start + 2 + length].trim();
        text.push_str(&rest[..start]);
        match values.iter().find(|(value_name, _)| *value_name == name) {
            Some((_, value)) => text.push_str(value),
            None => text.push_str(placeholder),
        }
        rest = &rest[start + placeholder.len()..];
    }
    text.push_str(rest);
    text
}

#[cfg(test)]
mod po_file;

#[cfg(test)]
mod tests;
