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
mod tests {
    use super::to_java_properties;
    use std::collections::BTreeMap;

    fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn writes_one_line_per_entry_in_a_stable_order() {
        let block = to_java_properties(&params(&[
            ("signaturePage", "3"),
            ("signatureSubFilter", "ETSI.CAdES.detached"),
        ]));

        assert_eq!(
            block,
            "signaturePage=3\nsignatureSubFilter=ETSI.CAdES.detached\n"
        );
    }

    #[test]
    fn writes_nothing_for_no_entries() {
        assert_eq!(to_java_properties(&BTreeMap::new()), "");
    }

    #[test]
    fn folds_the_newlines_of_the_layer2_text_into_one_line() {
        let block = to_java_properties(&params(&[(
            "layer2Text",
            "Firmado por: ADA LOVELACE BYRON - ***9999**.\nMotivo: Conforme",
        )]));

        assert_eq!(
            block,
            "layer2Text=Firmado por: ADA LOVELACE BYRON - ***9999**.\\nMotivo: Conforme\n"
        );
        assert_eq!(block.lines().count(), 1);
    }

    #[test]
    fn escapes_the_backslash_before_anything_else() {
        let block = to_java_properties(&params(&[("signReason", "C:\\nada")]));

        assert_eq!(block, "signReason=C:\\\\nada\n");
    }

    #[test]
    fn escapes_the_carriage_return_too() {
        let block = to_java_properties(&params(&[("layer2Text", "uno\r\ndos")]));

        assert_eq!(block, "layer2Text=uno\\r\\ndos\n");
    }

    #[test]
    fn writes_the_accents_as_ascii_escapes() {
        let block = to_java_properties(&params(&[("signReason", "Ratificación")]));

        assert_eq!(block, "signReason=Ratificaci\\u00F3n\n");
        assert!(block.is_ascii(), "el bloque tiene que ser ASCII puro");
    }

    #[test]
    fn writes_a_character_outside_the_basic_plane_as_two_escapes() {
        let block = to_java_properties(&params(&[("signReason", "\u{1F58A}")]));

        assert_eq!(block, "signReason=\\uD83D\\uDD8A\n");
    }

    #[test]
    fn leaves_the_base64_of_the_rubric_untouched() {
        let rubric = "/9j/4AAQSkZJRgABAQEAYABgAAD+abc=";
        let block = to_java_properties(&params(&[("signatureRubricImage", rubric)]));

        assert_eq!(block, format!("signatureRubricImage={rubric}\n"));
    }

    #[test]
    fn escapes_the_separators_when_they_are_in_a_key() {
        let block = to_java_properties(&params(&[("una clave=rara", "valor")]));

        assert_eq!(block, "una\\ clave\\=rara=valor\n");
    }

    #[test]
    fn leaves_the_separators_alone_when_they_are_in_a_value() {
        let block = to_java_properties(&params(&[("signReason", "a=b: c")]));

        assert_eq!(block, "signReason=a=b: c\n");
    }
}
