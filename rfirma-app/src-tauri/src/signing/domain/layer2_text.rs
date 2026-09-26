//! Composición del texto de rúbrica en capa 2 para la firma visible.

use serde::{Deserialize, Serialize};

use super::language::Language;

/// Carácter sustitutivo de la máscara. Es el de AutoFirma por omisión.
const OBFUSCATED_CHAR: char = '*';

/// Posiciones de la máscara por omisión: true se ve, false se oculta.
const MASK_POSITIONS: [bool; 7] = [false, false, false, true, true, true, true];

/// Dígitos seguidos necesarios para considerar un texto enmascarable.
const MIN_DIGITS: usize = 3;

/// Dígitos del cuerpo de un DNI, NIE o CIF.
const IDENTIFIER_DIGITS: std::ops::RangeInclusive<usize> = 7..=8;

/// Campos marcados en el panel de firma visible.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisibleTextFields<'a> {
    /// Nombre del firmante.
    pub signer_name: Option<&'a str>,
    /// Autoridad emisora del certificado.
    pub issuer: Option<&'a str>,
    /// Fecha y hora de la firma ya formateadas.
    pub signed_at: Option<&'a str>,
    /// Motivo de la firma.
    pub reason: Option<&'a str>,
    /// Indica si el certificado es de seudónimo.
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
    let VisibleTextFields {
        signer_name,
        issuer,
        signed_at,
        reason,
        pseudonym,
    } = fields;

    let paragraph = paragraph_of([
        (
            labels.signer,
            signer_name.map(|name| masked_signer(name, *pseudonym)),
        ),
        (labels.issuer, issuer.map(str::to_owned)),
        (labels.signed_at, signed_at.map(str::to_owned)),
    ]);

    match reason.map(|reason| format!("{}: {reason}", labels.reason)) {
        Some(reason) if paragraph.is_empty() => reason,
        Some(reason) => format!("{paragraph}\n{reason}"),
        None => paragraph,
    }
}

/// El contenido de la firma visible, elegido por modelo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibleContent {
    /// Firmante, fecha y emisor.
    Complete,
    /// Solo la imagen de la rúbrica, sin texto.
    RubricOnly,
    /// La frase que compuso la persona, con sus datos.
    Custom(Vec<PhrasePart>),
}

/// Un trozo de la frase de *Personalizada*: texto literal o un dato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhrasePart {
    Text(String),
    Datum(Datum),
}

/// Un dato que la firma visible toma del certificado o de la firma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Datum {
    Signer,
    Issuer,
    SignedAt,
}

/// Los valores de los datos para una firma concreta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisibleData<'a> {
    /// El `CN` del firmante, sin enmascarar.
    pub signer_name: &'a str,
    /// El `CN` del emisor del certificado.
    pub issuer: &'a str,
    /// La fecha y hora de la firma, ya formateadas.
    pub signed_at: &'a str,
    /// Si el certificado es de seudónimo, cuyo nombre no se enmascara.
    pub pseudonym: bool,
}

impl VisibleData<'_> {
    fn value_of(&self, datum: Datum) -> String {
        match datum {
            Datum::Signer => masked_signer(self.signer_name, self.pseudonym),
            Datum::Issuer => self.issuer.to_owned(),
            Datum::SignedAt => self.signed_at.to_owned(),
        }
    }
}

/// Compone el texto de la firma visible de un modelo, sin comodines que el puente pueda sustituir.
pub fn compose_visible_content(
    content: &VisibleContent,
    data: &VisibleData<'_>,
    language: Language,
) -> String {
    let text = match content {
        VisibleContent::Complete => complete_text(data, language),
        VisibleContent::RubricOnly => String::new(),
        VisibleContent::Custom(phrase) => phrase
            .iter()
            .map(|part| match part {
                PhrasePart::Text(text) => text.clone(),
                PhrasePart::Datum(datum) => data.value_of(*datum),
            })
            .collect(),
    };
    without_placeholders(text)
}

fn complete_text(data: &VisibleData<'_>, language: Language) -> String {
    let labels = labels(language);
    let present = |datum| Some(data.value_of(datum)).filter(|value| !value.is_empty());
    paragraph_of([
        (labels.signer, present(Datum::Signer)),
        (labels.signed_at, present(Datum::SignedAt)),
        (labels.issuer, present(Datum::Issuer)),
    ])
}

fn paragraph_of(sentences: [(&str, Option<String>); 3]) -> String {
    let mut paragraph = sentences
        .into_iter()
        .filter_map(|(label, value)| value.map(|value| format!("{label}: {value}")))
        .collect::<Vec<_>>()
        .join(". ");
    if !paragraph.is_empty() {
        paragraph.push('.');
    }
    paragraph
}

/// El firmante tal y como se estampa: enmascarado, salvo si es un seudónimo.
pub fn masked_signer(name: &str, pseudonym: bool) -> String {
    if pseudonym {
        name.to_owned()
    } else {
        obfuscate_ids(name)
    }
}

/// El puente sustituiría un `$$SUBJECTCN$$` escrito a mano sin pasar por la máscara (ADR-0006).
fn without_placeholders(mut text: String) -> String {
    while text.contains("$$") {
        text = text.replace("$$", "$");
    }
    text
}

/// Enmascara los identificadores dentro de un texto.
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

/// Comprueba si un fragmento alfanumérico encaja con un identificador.
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

/// Enmascara un identificador con la máscara por omisión.
pub fn mask_id_number(id: &str) -> String {
    let mut chars: Vec<char> = id.chars().collect();
    let digits = chars.iter().filter(|c| c.is_ascii_digit()).count();

    let mut digit_run = 0usize;
    let mut segment_start = 0usize;
    let mut found = false;
    for index in 0..chars.len() {
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

/// Aplica la máscara a un segmento.
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
mod tests;
