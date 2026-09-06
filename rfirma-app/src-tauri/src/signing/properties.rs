//! Serialización de parámetros extra a formato java.util.Properties en ASCII.

use std::collections::BTreeMap;

/// Escribe los `extraParams` como un bloque `java.util.Properties` en ASCII.
pub fn to_java_properties(params: &BTreeMap<String, String>) -> String {
    let mut block = String::new();
    for (key, value) in params {
        block.push_str(&escape(key, true));
        block.push('=');
        block.push_str(&escape(value, false));
        block.push('\n');
    }
    block
}

/// Escapa caracteres especiales para el formato java.util.Properties.
fn escape(text: &str, is_key: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '=' | ':' | ' ' if is_key => {
                escaped.push('\\');
                escaped.push(character);
            }
            character if character.is_ascii_graphic() || character == ' ' => {
                escaped.push(character)
            }
            character => {
                let mut buffer = [0u16; 2];
                for unit in character.encode_utf16(&mut buffer) {
                    escaped.push_str(&format!("\\u{unit:04X}"));
                }
            }
        }
    }
    escaped
}

#[cfg(test)]
mod tests;
