//! Los `extraParams` en el formato que el puente sabe leer: un bloque
//! `java.util.Properties`.
//!
//! [`SignatureConfig::extra_params`](super::config::SignatureConfig::extra_params)
//! decide **qué** claves se envían; este módulo decide **cómo** se escriben. La
//! separación no es cosmética: lo que se envía es una decisión del ID-18, y
//! cómo se escapa es una propiedad del lector de Java.
//!
//! # Dos trampas, y las dos son silenciosas
//!
//! 1. **Un salto de línea parte el bloque.** `layer2Text` es multilínea por
//!    construcción —una línea por casilla marcada—, así que sin escapar, la
//!    segunda línea se leería como una clave nueva. Se escapan `\`, el salto de
//!    línea y el retorno de carro, igual que hace `SessionStamp.escape` al otro
//!    lado.
//! 2. **`Properties.load(InputStream)` lee ISO-8859-1, no UTF-8.** Lo dice el
//!    javadoc de `java.util.Properties` y `NativeBridge.padesPreSign` le entrega
//!    los bytes UTF-8 de la cadena C, así que una «ó» de un motivo o de un
//!    apellido llegaría partida en dos caracteres y se estamparía así en el PDF.
//!    Por eso todo lo que no es ASCII sale como `\uXXXX`, que es el escape que
//!    ese mismo lector deshace: el bloque que emitimos es ASCII puro y las dos
//!    codificaciones coinciden sobre él.

use std::collections::BTreeMap;

/// Escribe los `extraParams` como un bloque `java.util.Properties` en ASCII.
///
/// El orden lo pone el `BTreeMap`: la misma configuración produce siempre el
/// mismo bloque, que es lo que permite compararlo en una prueba.
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

/// Escapa un trozo del bloque.
///
/// En una **clave** hay que escapar además los tres separadores (`=`, `:` y el
/// espacio), porque ahí terminan la clave; en un **valor** no significan nada.
/// Ninguna de las nueve claves del ID-18 los lleva, y se escapan igualmente:
/// que la corrección dependa de que nadie invente una décima clave con un
/// espacio dentro es justo la clase de suposición que se rompe callada.
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
            // El resto del ASCII imprimible viaja tal cual; lo demás, en
            // `\uXXXX`, que es lo que el lector ISO-8859-1 de Java deshace.
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
        // Sin esto, la segunda línea del recuadro se leería como otra clave.
        let block = to_java_properties(&params(&[(
            "layer2Text",
            "Firmado por: Ada Lovelace\nDNI: ***4567**",
        )]));

        assert_eq!(
            block,
            "layer2Text=Firmado por: Ada Lovelace\\nDNI: ***4567**\n"
        );
        assert_eq!(block.lines().count(), 1);
    }

    #[test]
    fn escapes_the_backslash_before_anything_else() {
        // Al revés, «\» + «n» se leería como el salto de línea que no era.
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
        // `Properties.load(InputStream)` lee ISO-8859-1: los bytes UTF-8 de una
        // «ó» llegarían como dos caracteres y se estamparían así en el PDF.
        let block = to_java_properties(&params(&[("signReason", "Ratificación")]));

        assert_eq!(block, "signReason=Ratificaci\\u00F3n\n");
        assert!(block.is_ascii(), "el bloque tiene que ser ASCII puro");
    }

    #[test]
    fn writes_a_character_outside_the_basic_plane_as_two_escapes() {
        // `\uXXXX` es de 16 bits: fuera del plano básico van los dos
        // subrogados, que es lo que el lector de Java vuelve a juntar.
        let block = to_java_properties(&params(&[("signReason", "\u{1F58A}")]));

        assert_eq!(block, "signReason=\\uD83D\\uDD8A\n");
    }

    #[test]
    fn leaves_the_base64_of_the_rubric_untouched() {
        // Base64 no lleva ni «\» ni saltos: si algún día lo tocáramos,
        // estaríamos corrompiendo la imagen sin que nadie viera un error.
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
