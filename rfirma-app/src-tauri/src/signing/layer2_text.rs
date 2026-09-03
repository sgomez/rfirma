//! El texto del recuadro de la firma visible, redactado por rFirma.
//!
//! **No hay comodines**: `$$SUBJECTCN$$` y `$$SIGNDATE$$` no salen de
//! aquí. Componer el texto entero en Rust y enviarlo ya resuelto en
//! `layer2Text` es lo que permite que siga al idioma de la aplicación, y lo que
//! deja fuera las rarezas del original —entre ellas que `$$PSEUDONYM$$`
//! estampe su literal en el PDF cuando el certificado no lleva el OID
//! `2.5.4.65`—.
//!
//! **Es un solo párrafo**, con las frases separadas por puntos y sin saltos de
//! línea forzados: iText reparte el tamaño de letra entre `alto del recuadro /
//! número de líneas`, así que menos líneas es letra más grande. El **motivo**
//! es la excepción y va en su propio renglón: es texto libre, puede ser largo,
//! y dentro del párrafo encogería la letra de todo lo demás.

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

/// Cuántos dígitos lleva el cuerpo de un DNI, un NIE o un CIF, ya sin la letra
/// inicial y sin el carácter de control del final.
const IDENTIFIER_DIGITS: std::ops::RangeInclusive<usize> = 7..=8;

/// Lo que el usuario ha marcado en las casillas del panel de firma.
///
/// `None` es «la casilla está sin marcar»; el dato no aparece en el recuadro.
/// La rúbrica no está aquí porque es una imagen, no texto.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisibleTextFields<'a> {
    /// El `CN` del subject **entero y en claro**, nombre y DNI juntos, que es
    /// como lo enseña AutoFirma. La máscara la aplica el compositor: llega
    /// aquí sin tocar.
    pub signer_name: Option<&'a str>,
    /// La autoridad emisora, la misma que se enseña en el desplegable.
    pub issuer: Option<&'a str>,
    /// Fecha y hora de la firma, **ya formateadas** por quien llama. Tiene que
    /// ser el mismo instante que acabará dentro del sello de sesión: el
    /// recuadro se estampa antes de la prefirma y el PDF ya no se vuelve a
    /// tocar.
    pub signed_at: Option<&'a str>,
    /// Motivo de la firma.
    pub reason: Option<&'a str>,
    /// Si el certificado es **de seudónimo**: entonces el `CN` se estampa sin
    /// enmascarar, como hace el original (`PdfSessionManager.java:206-214`).
    pub pseudonym: bool,
}

/// Las etiquetas del recuadro en un idioma.
struct Layer2Labels {
    signer: &'static str,
    issuer: &'static str,
    signed_at: &'static str,
    reason: &'static str,
}

fn labels(language: Language) -> Layer2Labels {
    match language {
        Language::Spanish => Layer2Labels {
            signer: "Firmado por",
            issuer: "Emisor",
            signed_at: "Fecha",
            reason: "Motivo",
        },
        Language::Catalan => Layer2Labels {
            signer: "Signat per",
            issuer: "Emissor",
            signed_at: "Data",
            reason: "Motiu",
        },
        Language::Basque => Layer2Labels {
            signer: "Sinatzailea",
            issuer: "Jaulkitzailea",
            signed_at: "Data",
            reason: "Arrazoia",
        },
        Language::Galician => Layer2Labels {
            signer: "Asinado por",
            issuer: "Emisor",
            signed_at: "Data",
            reason: "Motivo",
        },
        Language::English => Layer2Labels {
            signer: "Signed by",
            issuer: "Issuer",
            signed_at: "Date",
            reason: "Reason",
        },
    }
}

/// Compone el texto del recuadro con las casillas marcadas, en el idioma de la
/// aplicación.
///
/// Firmante, emisor y fecha van en **un solo párrafo**, separados por puntos;
/// el motivo, si lo hay, en el renglón de debajo. Sin ninguna casilla marcada
/// devuelve la cadena vacía, que **no** es lo mismo que no enviar `layer2Text`:
/// ver [`super::config::SignatureConfig`].
pub fn compose_layer2_text(fields: &VisibleTextFields<'_>, language: Language) -> String {
    let labels = labels(language);
    // Destructurado exhaustivo: una casilla nueva no compila hasta que alguien
    // decida en qué parte del recuadro cae.
    let VisibleTextFields {
        signer_name,
        issuer,
        signed_at,
        reason,
        pseudonym,
    } = fields;

    let signer = signer_name.map(|name| {
        if *pseudonym {
            name.to_owned()
        } else {
            obfuscate_ids(name)
        }
    });

    let sentences = [
        (labels.signer, signer),
        (labels.issuer, issuer.map(str::to_owned)),
        (labels.signed_at, signed_at.map(str::to_owned)),
    ];

    let mut paragraph = sentences
        .into_iter()
        .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
        .collect::<Vec<_>>()
        .join(". ");
    if !paragraph.is_empty() {
        paragraph.push('.');
    }

    match reason.map(|reason| format!("{}: {reason}", labels.reason)) {
        Some(reason) if paragraph.is_empty() => reason,
        Some(reason) => format!("{paragraph}\n{reason}"),
        None => paragraph,
    }
}

/// Enmascara los identificadores que lleve dentro un texto, que es como el
/// original tapa el DNI: sobre el `CN`, no sobre un campo aparte
/// (`PdfVisibleAreasUtils.getLayerText:262-267`).
///
/// Los certificados españoles llevan el DNI dentro del `CN` —«ADA LOVELACE
/// BYRON - 99999999R»—, así que enmascarar ahí es enmascararlo donde de verdad
/// está. Se parte el texto en fragmentos alfanuméricos y solo se enmascaran
/// los que **encajan con el patrón** de DNI, NIE o CIF: nadie se apellida como
/// un DNI, y ceñirse al patrón deja el resto del nombre intacto.
///
/// Aquí está la diferencia con el original, y es a propósito: él le pasa a
/// `countDigits` la cadena entera (`PdfVisibleAreasUtils.java:707`), de modo
/// que sobre un nombre completo el recuento decide por una rama que no es la
/// del fragmento que enmascara. Como aquí la máscara se aplica **al fragmento
/// solo**, el recuento es el suyo y la rama la correcta.
pub fn obfuscate_ids(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut obfuscated = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < chars.len() {
        if !chars[index].is_alphanumeric() {
            obfuscated.push(chars[index]);
            index += 1;
            continue;
        }
        let start = index;
        while index < chars.len() && chars[index].is_alphanumeric() {
            index += 1;
        }
        let fragment: String = chars[start..index].iter().collect();
        if looks_like_an_identifier(&fragment) {
            obfuscated.push_str(&mask_id_number(&fragment));
        } else {
            obfuscated.push_str(&fragment);
        }
    }
    obfuscated
}

/// Si un fragmento alfanumérico es un DNI, un NIE o un CIF.
///
/// Los tres son el mismo esqueleto: una letra inicial opcional —la `X`, `Y` o
/// `Z` del NIE, la de tipo de un CIF—, siete u ocho dígitos, y un carácter de
/// control que puede ser letra o dígito. Un número de teléfono o un año no
/// entran, que es lo que hace seguro aplicar esto sobre un nombre.
fn looks_like_an_identifier(fragment: &str) -> bool {
    if !fragment.is_ascii() {
        return false;
    }
    let mut body = fragment;
    if body.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        body = &body[1..];
    }
    if body.chars().last().is_some_and(|c| c.is_ascii_alphabetic()) {
        body = &body[..body.len() - 1];
    }
    IDENTIFIER_DIGITS.contains(&body.len()) && body.chars().all(|c| c.is_ascii_digit())
}

/// Enmascara un identificador con la máscara por omisión de AutoFirma.
///
/// **Es cosmética.** El certificado viaja entero dentro de la firma con el DNI
/// en claro, y cualquier lector de PDF lo enseña al inspeccionarla: esto
/// protege de la lectura casual del recuadro, no del documento. Por eso no
/// tiene interruptor —se aplica siempre— y por eso tampoco se apoya en
/// `obfuscateCertText`, que solo actúa al sustituir comodines y aquí no hay.
///
/// Replica `PdfVisibleAreasUtils.obfuscateIds:660-691` con la máscara por
/// omisión de `PdfTextMask`, **incluida la segmentación**: el texto se parte
/// por los caracteres que no son alfanuméricos y la máscara cae solo sobre el
/// segmento que contiene la racha de dígitos. Así `IDCES-99999999R` —la forma
/// en la que el RDN `serialNumber` de la FNMT y del DNIe trae el número— sale
/// como `IDCES-***9999**` y no como un borrón entero de asteriscos.
///
/// Con sus tres rarezas, que se replican y no se corrigen:
///
/// 1. Las letras del segmento se ocultan siempre (por eso `99999999R` acaba
///    en `**`).
/// 2. Un segmento con menos dígitos que posiciones visibles se enmascara
///    **desde atrás**, que deja los dígitos intactos y oculta lo de delante.
/// 3. El recuento de dígitos que decide entre las dos ramas es el de **toda
///    la cadena**, no el del segmento (`PdfVisibleAreasUtils.java:707` le pasa
///    a `countDigits` el array entero). Es incoherente con el resto del
///    algoritmo, pero es lo que estampa AutoFirma en el PDF.
pub fn mask_id_number(id: &str) -> String {
    let mut chars: Vec<char> = id.chars().collect();
    // Rareza 3: el recuento es global aunque el enmascarado sea por segmento.
    let digits = chars.iter().filter(|c| c.is_ascii_digit()).count();

    // El bucle exterior de `obfuscateIds`. `digit_run` se reinicia con las
    // letras y **no** con los separadores, igual que el `digitCount` de Java;
    // `found` sobrevive a las letras que vengan detrás de la racha.
    let mut digit_run = 0usize;
    let mut segment_start = 0usize;
    let mut found = false;
    for index in 0..chars.len() {
        // `Character.isLetterOrDigit` es Unicode: una eñe o una tilde no
        // parten el segmento. Dígito sí se lee en ASCII: un DNI o un NIE no
        // traen otra cosa.
        if chars[index].is_alphanumeric() {
            if chars[index].is_ascii_digit() {
                digit_run += 1;
                if digit_run == MIN_DIGITS {
                    found = true;
                }
            } else {
                digit_run = 0;
            }
        } else {
            if found {
                obfuscate(&mut chars, segment_start, index - segment_start, digits);
                found = false;
            }
            segment_start = index + 1;
        }
    }
    if found {
        let length = chars.len() - segment_start;
        obfuscate(&mut chars, segment_start, length, digits);
    }

    chars.into_iter().collect()
}

/// Aplica la máscara a un segmento, con `digits` contados sobre la cadena
/// entera. Es `PdfVisibleAreasUtils.obfuscate:704-761`.
fn obfuscate(chars: &mut [char], start: usize, length: usize, digits: usize) {
    let plain = MASK_POSITIONS.iter().filter(|visible| **visible).count();
    let segment = &mut chars[start..start + length];

    if digits >= plain {
        let positions = fitted_positions(digits);
        let mut position = 0usize;
        for c in segment {
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
        for (offset, c) in segment.iter_mut().rev().enumerate() {
            let position = MASK_POSITIONS.len().checked_sub(offset + 1);
            let visible = position.is_some_and(|p| MASK_POSITIONS[p]);
            if !visible {
                *c = OBFUSCATED_CHAR;
            }
        }
    }
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
    use super::{compose_layer2_text, mask_id_number, obfuscate_ids, VisibleTextFields};
    use crate::signing::language::Language;

    fn all_fields<'a>() -> VisibleTextFields<'a> {
        VisibleTextFields {
            signer_name: Some("ADA LOVELACE BYRON - 99999999R"),
            issuer: Some("AC FNMT Usuarios"),
            signed_at: Some("31/08/2026 12:00:00 CEST"),
            reason: Some("Conforme"),
            pseudonym: false,
        }
    }

    #[test]
    fn composes_one_paragraph_and_leaves_the_reason_on_its_own_line() {
        assert_eq!(
            compose_layer2_text(&all_fields(), Language::Spanish),
            "Firmado por: ADA LOVELACE BYRON - ***9999**. \
             Emisor: AC FNMT Usuarios. \
             Fecha: 31/08/2026 12:00:00 CEST.\n\
             Motivo: Conforme"
        );
    }

    #[test]
    fn forces_no_line_break_other_than_the_one_before_the_reason() {
        let without_reason = VisibleTextFields {
            reason: None,
            ..all_fields()
        };
        assert!(!compose_layer2_text(&without_reason, Language::Spanish).contains('\n'));
        assert_eq!(
            compose_layer2_text(&all_fields(), Language::Spanish)
                .lines()
                .count(),
            2
        );
    }

    #[test]
    fn composes_only_the_reason_when_it_is_the_only_box_checked() {
        let fields = VisibleTextFields {
            reason: Some("Conforme"),
            ..VisibleTextFields::default()
        };
        assert_eq!(
            compose_layer2_text(&fields, Language::Spanish),
            "Motivo: Conforme"
        );
    }

    #[test]
    fn drops_the_unchecked_fields() {
        let fields = VisibleTextFields {
            signer_name: Some("ADA LOVELACE BYRON"),
            ..VisibleTextFields::default()
        };
        assert_eq!(
            compose_layer2_text(&fields, Language::Spanish),
            "Firmado por: ADA LOVELACE BYRON."
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
                text.contains("ADA LOVELACE BYRON"),
                "falta el titular en {}",
                language.tag()
            );
            if language != Language::Spanish {
                assert_ne!(text, spanish, "{} no traduce nada", language.tag());
            }
        }
    }

    #[test]
    fn masks_the_id_inside_the_common_name_without_a_switch() {
        let fields = VisibleTextFields {
            signer_name: Some("ADA LOVELACE BYRON - 99999999R"),
            ..VisibleTextFields::default()
        };
        let text = compose_layer2_text(&fields, Language::Spanish);
        assert!(!text.contains("99999999R"), "el DNI sale en claro: {text}");
        assert!(text.contains("***9999**"), "{text}");
    }

    #[test]
    fn a_pseudonym_certificate_is_exempt_from_the_mask() {
        let fields = VisibleTextFields {
            signer_name: Some("SEUDONIMO 99999999R"),
            pseudonym: true,
            ..VisibleTextFields::default()
        };
        assert_eq!(
            compose_layer2_text(&fields, Language::Spanish),
            "Firmado por: SEUDONIMO 99999999R."
        );
    }

    /// Los cuatro formatos españoles que llegan en el `CN`, con el DNI o el
    /// CIF dentro: se tapa el identificador y **solo** el identificador.
    #[test]
    fn masks_the_identifier_of_every_spanish_common_name() {
        // FNMT de persona física.
        assert_eq!(
            obfuscate_ids("ADA LOVELACE BYRON - 99999999R"),
            "ADA LOVELACE BYRON - ***9999**"
        );
        // Empleado público.
        assert_eq!(
            obfuscate_ids("ADA LOVELACE BYRON - NIF 99999999R"),
            "ADA LOVELACE BYRON - NIF ***9999**"
        );
        // Representante de empresa: el NIE de la persona y el CIF de la
        // sociedad, los dos tapados.
        assert_eq!(
            obfuscate_ids("X1234567L - EMPRESA EJEMPLO SL - A12345674"),
            "****4567* - EMPRESA EJEMPLO SL - ****4567*"
        );
        // DNIe: su `CN` viene con la coma ya desescapada y sin identificador.
        assert_eq!(
            obfuscate_ids("APELLIDO1 APELLIDO2, ADA (FIRMA)"),
            "APELLIDO1 APELLIDO2, ADA (FIRMA)"
        );
    }

    #[test]
    fn leaves_alone_what_is_not_an_identifier() {
        assert_eq!(obfuscate_ids("ADA LOVELACE BYRON"), "ADA LOVELACE BYRON");
        // Un teléfono tiene nueve dígitos y un año cuatro: ninguno encaja.
        assert_eq!(obfuscate_ids("600123456 y 2026"), "600123456 y 2026");
        assert_eq!(obfuscate_ids("ANDRÉS PEÑA"), "ANDRÉS PEÑA");
        assert_eq!(obfuscate_ids(""), "");
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
    fn masks_only_the_segment_that_holds_the_digits() {
        assert_eq!(mask_id_number("IDCES-99999999R"), "IDCES-***9999**");
        assert_eq!(mask_id_number("12345678-Z"), "***4567*-Z");
        assert_eq!(mask_id_number("99999999 R"), "***9999* R");
    }

    #[test]
    fn keeps_the_digit_run_across_a_separator_like_java_does() {
        // `digitCount` de Java solo se reinicia con letras, así que `12-345`
        // sí llega a la racha mínima; el segmento enmascarado es el segundo.
        assert_eq!(mask_id_number("12-345"), "12-*45");
    }

    /// La rareza 3 del original —el recuento de dígitos sobre toda la cadena—
    /// no llega al recuadro: la máscara la aplica [`obfuscate_ids`] sobre el
    /// fragmento suelto, donde el recuento es el del propio identificador.
    #[test]
    fn counts_the_digits_of_the_identifier_and_not_those_of_the_whole_name() {
        assert_eq!(
            obfuscate_ids("ADA 12 LOVELACE 345 BYRON - 99999999R"),
            "ADA 12 LOVELACE 345 BYRON - ***9999**"
        );
    }
}
