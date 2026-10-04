//! La lectura de un `.po` en pares clave → texto, con las reglas de `po-import`; la comparten `build.rs` y las pruebas.

/// Los sufijos de plural, en el orden de los `msgstr[n]`: los del castellano, en los cinco idiomas.
pub const PLURAL_SUFFIXES: [&str; 3] = ["one", "many", "other"];

#[derive(Default)]
struct Entry {
    msgid: String,
    plural: bool,
    texts: Vec<String>,
    fuzzy: bool,
    obsolete: bool,
}

enum Field {
    Id,
    Plural,
    Text(usize),
}

/// Las entradas de un `.po` como pares clave → texto; un `#, fuzzy` cuenta como no traducido.
pub fn entries_of(po: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut entry = Entry::default();
    let mut field = None;
    for line in po.lines().map(str::trim) {
        if line.is_empty() {
            push_entry(&mut entries, std::mem::take(&mut entry));
            field = None;
        } else if line.starts_with("#~") {
            entry.obsolete = true;
        } else if let Some(flags) = line.strip_prefix("#,") {
            entry.fuzzy |= flags.split(',').any(|flag| flag.trim() == "fuzzy");
        } else if line.starts_with("msgid_plural ") {
            entry.plural = true;
            field = Some(Field::Plural);
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            entry.msgid = unquoted(rest);
            field = Some(Field::Id);
        } else if let Some(rest) = line.strip_prefix("msgstr[") {
            let (index, rest) = rest.split_once(']').unwrap_or(("0", rest));
            let index = index.parse().unwrap_or(0);
            text_at(&mut entry, index).push_str(&unquoted(rest.trim_start()));
            field = Some(Field::Text(index));
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            text_at(&mut entry, 0).push_str(&unquoted(rest));
            field = Some(Field::Text(0));
        } else if line.starts_with('"') {
            let continued = unquoted(line);
            match field {
                Some(Field::Id) => entry.msgid.push_str(&continued),
                Some(Field::Text(index)) => text_at(&mut entry, index).push_str(&continued),
                Some(Field::Plural) | None => {}
            }
        }
    }
    push_entry(&mut entries, entry);
    entries
}

/// Un catálogo está completo cuando ninguna de sus cadenas está vacía.
pub fn is_complete(entries: &[(String, String)]) -> bool {
    entries.iter().all(|(_, text)| !text.trim().is_empty())
}

fn text_at(entry: &mut Entry, index: usize) -> &mut String {
    if entry.texts.len() <= index {
        entry.texts.resize(index + 1, String::new());
    }
    &mut entry.texts[index]
}

fn push_entry(entries: &mut Vec<(String, String)>, entry: Entry) {
    if entry.obsolete || entry.msgid.is_empty() {
        return;
    }
    let text_of = |index: usize| {
        if entry.fuzzy {
            String::new()
        } else {
            entry.texts.get(index).cloned().unwrap_or_default()
        }
    };
    if !entry.plural {
        entries.push((entry.msgid.clone(), text_of(0)));
        return;
    }
    for (index, suffix) in PLURAL_SUFFIXES.iter().enumerate() {
        entries.push((format!("{}_{suffix}", entry.msgid), text_of(index)));
    }
}

fn unquoted(literal: &str) -> String {
    let inner = literal
        .trim()
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_default();
    let mut text = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            text.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => text.push('\n'),
            Some('t') => text.push('\t'),
            Some('r') => text.push('\r'),
            Some(other) => text.push(other),
            None => text.push('\\'),
        }
    }
    text
}
