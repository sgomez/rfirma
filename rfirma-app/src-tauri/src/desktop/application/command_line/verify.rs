//! La orden `verify`: valida las firmas de un fichero y deja lo que el original imprime de cada una, no el XML de `-xml`.

use std::path::Path;

use super::{CommandLinePorts, Outcome};
use crate::desktop::domain::command_line::{documented, value_of, Refusal, INPUT, XML};
use crate::signing::domain::bridge::{Format, XadesVariant};
use crate::site::domain::protocol::detection::{is_cms_signed_data, shape_of, DetectedShape};

/// Lo que el original imprime de unos datos que no son de ningún formato de firma que reconozca.
pub const UNKNOWN_FORMAT: &str = "Firma no valida: los datos proporcionados no se corresponden \
                                  con ningún formato de firma reconocido";

pub(super) fn verify(arguments: &[String], ports: &CommandLinePorts) -> Outcome {
    if arguments.iter().any(|argument| argument == XML) {
        return Outcome::failed(format!(
            "rfirma: el parámetro {} de «verify» todavía no está disponible en esta versión",
            documented(XML)
        ));
    }
    let Some(input) = value_of(arguments, INPUT) else {
        return Outcome::refused(&Refusal::MissingParameter(INPUT));
    };
    let document = match ports.files.read(Path::new(input)) {
        Ok(document) => document,
        Err(detail) => {
            return Outcome::failed(format!("rfirma: no se puede leer «{input}» ({detail})"))
        }
    };
    let Some(format) = format_to_verify(&document) else {
        return Outcome::printed(&[UNKNOWN_FORMAT.to_owned()]);
    };
    match ports.verifier.results_of(&document, format) {
        Ok(results) => Outcome::printed(&results),
        Err(error) => Outcome::failed(format!(
            "rfirma: no se han podido validar las firmas de «{input}» ({error})"
        )),
    }
}

/// El formato de `-format auto` con cuyo validador se examinan los datos, o nada si no son una firma.
pub fn format_to_verify(document: &[u8]) -> Option<Format> {
    match shape_of(document) {
        DetectedShape::Pdf => Some(Format::Pades),
        DetectedShape::Invoice => Some(Format::FacturaE),
        DetectedShape::Xml => Some(Format::Xades(XadesVariant::Enveloping)),
        DetectedShape::Binary if is_cms_signed_data(document) => Some(Format::Cades),
        DetectedShape::Binary => None,
    }
}

#[cfg(test)]
mod tests;
