//! El texto del recuadro de la firma visible, redactado por rFirma.
//!
//! **No hay comodines** (ID-19): `$$SUBJECTCN$$` y `$$SIGNDATE$$` no salen de
//! aquí, y para el DNI AutoFirma no tiene ninguno —vive en el RDN
//! `serialNumber` y solo asoma pegado al nombre dentro de `$$SUBJECTCN$$`—, así
//! que separar «Nombre y apellidos» de «DNI» obliga a componer el texto entero
//! en Rust y enviarlo ya resuelto en `layer2Text`.

use super::language::Language;

/// Carácter sustitutivo de la máscara. Es el de AutoFirma por omisión.
const OBFUSCATED_CHAR: char = '*';

/// Posiciones de la máscara por omisión de AutoFirma: `true` se ve, `false` se
/// oculta. Tres ocultas y cuatro visibles, y todo lo que sobre por el final se
/// oculta también.
const MASK_POSITIONS: [bool; 7] = [false, false, false, true, true, true, true];

/// Dígitos seguidos que hacen falta para que un texto se considere un
/// identificador enmascarable.
const MIN_DIGITS: usize = 3;

/// Lo que el usuario ha marcado en las casillas del panel de firma.
///
/// `None` es «la casilla está sin marcar»; el dato no aparece en el recuadro.
/// La rúbrica no está aquí porque es una imagen, no texto.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisibleTextFields<'a> {
    /// Nombre y apellidos del titular.
    pub signer_name: Option<&'a str>,
    /// DNI o NIE **en claro**: la máscara la aplica el compositor, siempre.
    pub id_number: Option<&'a str>,
    /// Fecha y hora de la firma, **ya formateadas** por quien llama. Tiene que
    /// ser el mismo instante que acabará dentro del sello de sesión: el
    /// recuadro se estampa antes de la prefirma y el PDF ya no se vuelve a
    /// tocar.
    pub signed_at: Option<&'a str>,
    /// Motivo de la firma.
    pub reason: Option<&'a str>,
}

/// Las etiquetas del recuadro en un idioma.
struct Layer2Labels {
    signer: &'static str,
    id_number: &'static str,
    signed_at: &'static str,
    reason: &'static str,
}

fn labels(language: Language) -> Layer2Labels {
    match language {
        Language::Spanish => Layer2Labels {
            signer: "Firmado por",
            id_number: "DNI",
            signed_at: "Fecha",
            reason: "Motivo",
        },
        Language::Catalan => Layer2Labels {
            signer: "Signat per",
            id_number: "DNI",
            signed_at: "Data",
            reason: "Motiu",
        },
        Language::Basque => Layer2Labels {
            signer: "Sinatzailea",
            id_number: "NAN",
            signed_at: "Data",
            reason: "Arrazoia",
        },
        Language::Galician => Layer2Labels {
            signer: "Asinado por",
            id_number: "DNI",
            signed_at: "Data",
            reason: "Motivo",
        },
        Language::Valencian => Layer2Labels {
            signer: "Signat per",
            id_number: "DNI",
            signed_at: "Data",
            reason: "Motiu",
        },
        Language::English => Layer2Labels {
            signer: "Signed by",
            id_number: "ID number",
            signed_at: "Date",
            reason: "Reason",
        },
    }
}

/// Compone el texto del recuadro con las casillas marcadas, en el idioma de la
/// aplicación.
///
/// Sin ninguna casilla marcada devuelve la cadena vacía, que **no** es lo mismo
/// que no enviar `layer2Text`: ver [`super::config::SignatureConfig`].
pub fn compose_layer2_text(fields: &VisibleTextFields<'_>, language: Language) -> String {
    let labels = labels(language);
    // Destructurado exhaustivo: una casilla nueva no compila hasta que alguien
    // decida en qué línea del recuadro cae.
    let VisibleTextFields {
        signer_name,
        id_number,
        signed_at,
        reason,
    } = fields;

    let lines = [
        (labels.signer, signer_name.map(str::to_owned)),
        (labels.id_number, id_number.map(mask_id_number)),
        (labels.signed_at, signed_at.map(str::to_owned)),
        (labels.reason, reason.map(str::to_owned)),
    ];

    lines
        .into_iter()
        .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Enmascara un identificador con la máscara por omisión de AutoFirma.
///
/// **Es cosmética.** El certificado viaja entero dentro de la firma con el DNI
/// en claro, y cualquier lector de PDF lo enseña al inspeccionarla: esto
/// protege de la lectura casual del recuadro, no del documento. Por eso no
/// tiene interruptor —se aplica siempre— y por eso tampoco se apoya en
/// `obfuscateCertText`, que solo actúa al sustituir comodines y aquí no hay.
///
/// Replica `PdfVisibleAreasUtils.obfuscateIds` con la máscara por omisión de
/// `PdfTextMask`, incluidas sus dos rarezas: las letras del identificador
/// siempre se ocultan, y un identificador con menos dígitos que posiciones
/// visibles se enmascara desde atrás.
pub fn mask_id_number(id: &str) -> String {
    let mut chars: Vec<char> = id.chars().collect();
    if !has_digit_run(&chars) {
        return id.to_owned();
    }

    let digits = chars.iter().filter(|c| c.is_ascii_digit()).count();
    let plain = MASK_POSITIONS.iter().filter(|visible| **visible).count();

    if digits >= plain {
        let positions = fitted_positions(digits);
        let mut position = 0usize;
        for c in &mut chars {
            if c.is_ascii_digit() {
                if !positions.get(position).copied().unwrap_or(false) {
                    *c = OBFUSCATED_CHAR;
                }
                position += 1;
            } else {
                *c = OBFUSCATED_CHAR;
            }
        }
    } else {
        for (offset, c) in chars.iter_mut().rev().enumerate() {
            let position = MASK_POSITIONS.len().checked_sub(offset + 1);
            let visible = position.is_some_and(|p| MASK_POSITIONS[p]);
            if !visible {
                *c = OBFUSCATED_CHAR;
            }
        }
    }

    chars.into_iter().collect()
}

/// ¿Hay [`MIN_DIGITS`] dígitos seguidos? Es lo que AutoFirma exige para
/// considerar que un texto es un identificador.
fn has_digit_run(chars: &[char]) -> bool {
    let mut run = 0usize;
    for c in chars {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        if run >= MIN_DIGITS {
            return true;
        }
    }
    false
}

/// Adapta la máscara a un identificador con menos dígitos que posiciones,
/// omitiendo posiciones ocultas del principio para no comerse las visibles.
fn fitted_positions(digits: usize) -> Vec<bool> {
    if digits >= MASK_POSITIONS.len() {
        return MASK_POSITIONS.to_vec();
    }
    let omit = MASK_POSITIONS.len() - digits;
    let mut fitted = vec![false; digits];
    let mut omitted = 0usize;
    for (i, &visible) in MASK_POSITIONS.iter().enumerate() {
        if visible || omitted >= omit {
            fitted[i - omitted] = visible;
        } else {
            omitted += 1;
        }
    }
    fitted
}

#[cfg(test)]
mod tests {
    use super::{compose_layer2_text, mask_id_number, VisibleTextFields};
    use crate::signing::language::Language;

    fn all_fields<'a>() -> VisibleTextFields<'a> {
        VisibleTextFields {
            signer_name: Some("Ada Lovelace Byron"),
            id_number: Some("99999999R"),
            signed_at: Some("31/08/2026 12:00:00 CEST"),
            reason: Some("Conforme"),
        }
    }

    #[test]
    fn composes_one_line_per_checked_field() {
        assert_eq!(
            compose_layer2_text(&all_fields(), Language::Spanish),
            "Firmado por: Ada Lovelace Byron\n\
             DNI: ***9999**\n\
             Fecha: 31/08/2026 12:00:00 CEST\n\
             Motivo: Conforme"
        );
    }

    #[test]
    fn drops_the_unchecked_fields() {
        let fields = VisibleTextFields {
            signer_name: Some("Ada Lovelace Byron"),
            ..VisibleTextFields::default()
        };
        assert_eq!(
            compose_layer2_text(&fields, Language::Spanish),
            "Firmado por: Ada Lovelace Byron"
        );
    }

    #[test]
    fn composes_nothing_when_no_field_is_checked() {
        assert_eq!(
            compose_layer2_text(&VisibleTextFields::default(), Language::Spanish),
            ""
        );
    }

    #[test]
    fn never_emits_an_autofirma_wildcard() {
        for language in Language::ALL {
            let text = compose_layer2_text(&all_fields(), language);
            assert!(
                !text.contains("$$"),
                "el texto en {} lleva un comodín: {text}",
                language.tag()
            );
        }
    }

    #[test]
    fn follows_the_language_of_the_application() {
        let fields = all_fields();
        let spanish = compose_layer2_text(&fields, Language::Spanish);
        for language in Language::ALL {
            let text = compose_layer2_text(&fields, language);
            assert!(
                text.contains("Ada Lovelace Byron"),
                "falta el titular en {}",
                language.tag()
            );
            if language != Language::Spanish {
                assert_ne!(text, spanish, "{} no traduce nada", language.tag());
            }
        }
    }

    #[test]
    fn masks_the_id_number_without_a_switch() {
        let fields = VisibleTextFields {
            id_number: Some("99999999R"),
            ..VisibleTextFields::default()
        };
        let text = compose_layer2_text(&fields, Language::Spanish);
        assert!(!text.contains("99999999R"), "el DNI sale en claro: {text}");
        assert!(text.ends_with("***9999**"), "{text}");
    }

    #[test]
    fn masks_a_dni_like_autofirma_does() {
        assert_eq!(mask_id_number("99999999R"), "***9999**");
        assert_eq!(mask_id_number("12345678Z"), "***4567**");
    }

    #[test]
    fn masks_a_nie_like_autofirma_does() {
        assert_eq!(mask_id_number("X1234567L"), "****4567*");
    }

    #[test]
    fn shifts_the_mask_when_there_are_fewer_digits_than_positions() {
        assert_eq!(mask_id_number("12345"), "*2345");
        assert_eq!(mask_id_number("1234"), "1234");
    }

    #[test]
    fn masks_from_the_back_when_there_are_fewer_digits_than_visible_positions() {
        // La rareza de AutoFirma: con tres dígitos la máscara se aplica desde
        // atrás y los que se ocultan son los caracteres de delante.
        assert_eq!(mask_id_number("AB123"), "*B123");
    }

    #[test]
    fn leaves_alone_what_is_not_an_identifier() {
        assert_eq!(mask_id_number("Ada Lovelace"), "Ada Lovelace");
        assert_eq!(mask_id_number("A1B2C3"), "A1B2C3");
        assert_eq!(mask_id_number(""), "");
    }
}
