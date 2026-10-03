//! La orden `verify`: valida las firmas de un fichero y deja lo que el original imprime de cada una, no el XML de `-xml`.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::{CommandLinePorts, Outcome};
use crate::desktop::domain::command_line::{documented, value_of, verbosity, Refusal, INPUT, XML};
use crate::desktop::ports::LocalTimeZone;
use crate::identity::domain::holder::without_semantics_prefix;
use crate::signing::domain::bridge::{Format, XadesVariant};
use crate::signing::domain::DocumentSignature;
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
        Ok(results) if verbosity(arguments) > 0 => {
            with_the_signatures(results, &document, format, verbosity(arguments), ports)
        }
        Ok(results) => Outcome::printed(&results),
        Err(error) => Outcome::failed(format!(
            "rfirma: no se han podido validar las firmas de «{input}» ({error})"
        )),
    }
}

fn with_the_signatures(
    mut lines: Vec<String>,
    document: &[u8],
    format: Format,
    verbosity: usize,
    ports: &CommandLinePorts,
) -> Outcome {
    let signatures = match ports.reader.signatures_in(document) {
        Ok(signatures) => signatures,
        Err(error) => {
            let mut outcome = Outcome::printed(&lines);
            outcome.stderr.push(format!(
                "rfirma: no se han podido leer las firmas del documento: {error}"
            ));
            return outcome;
        }
    };
    lines.extend([String::new(), format!("Formato: {}", family_of(format))]);
    if signatures.count() == 0 {
        lines.extend([String::new(), "El documento no tiene firmas.".to_owned()]);
    }
    for (number, signature) in signatures.signatures().iter().enumerate() {
        let title = format!("Firma {}", number + 1);
        lines.extend(tree_of(signature, &title, 0, verbosity, ports.time_zone));
    }
    Outcome::printed(&lines)
}

fn family_of(format: Format) -> &'static str {
    match format {
        Format::Xades(_) => "XAdES",
        other => other.name(),
    }
}

/// La ficha de una firma y, indentadas dentro, las de sus contrafirmas numeradas «N.M».
fn tree_of(
    signature: &DocumentSignature,
    title: &str,
    depth: usize,
    verbosity: usize,
    time_zone: &dyn LocalTimeZone,
) -> Vec<String> {
    let indent = " ".repeat(4 * depth);
    let mut lines = vec![String::new(), format!("{indent}{title}")];
    lines.extend(
        sheet_of(signature, verbosity, time_zone)
            .into_iter()
            .map(|line| format!("{indent}{line}")),
    );
    let number = title.rsplit(' ').next().unwrap_or_default();
    for (index, countersignature) in signature.countersignatures.iter().enumerate() {
        let title = format!("Contrafirma {number}.{}", index + 1);
        lines.extend(tree_of(
            countersignature,
            &title,
            depth + 1,
            verbosity,
            time_zone,
        ));
    }
    lines
}

fn sheet_of(
    signature: &DocumentSignature,
    verbosity: usize,
    time_zone: &dyn LocalTimeZone,
) -> Vec<String> {
    let (signer, on_behalf_of) = signer_and_entity_of(signature);
    let issuer = Some(signature.issuer.clone()).filter(|issuer| !issuer.is_empty());
    let declared = signature
        .signing_time
        .as_deref()
        .map(|instant| in_local_time(instant, time_zone));
    let serial = Some(signature.certificate_serial_number.clone())
        .filter(|serial| verbosity > 1 && !serial.is_empty());
    [
        ("Firmante", signer),
        ("En nombre de", on_behalf_of),
        ("Emisor", issuer),
        ("Fecha declarada", declared),
        ("Número de serie", serial),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| format!("  {:<19}{value}", format!("{label}:"))))
    .collect()
}

fn signer_and_entity_of(signature: &DocumentSignature) -> (Option<String>, Option<String>) {
    let Some(identifier) = signature
        .organization_identifier
        .as_deref()
        .map(without_semantics_prefix)
    else {
        return (signer_of(signature), None);
    };
    let entity = || {
        named_with_id(
            signature.organization_name.as_deref().unwrap_or_default(),
            identifier,
        )
    };
    let id_number = without_semantics_prefix(&signature.id_number);
    if id_number.is_empty() || id_number == identifier {
        (entity(), None)
    } else {
        (representative_of(signature), entity())
    }
}

fn representative_of(signature: &DocumentSignature) -> Option<String> {
    let id_number = without_semantics_prefix(&signature.id_number);
    let name = signature.name.as_str();
    let name = name.strip_prefix(id_number).map_or(name, str::trim_start);
    let name = name.rfind(" (R: ").map_or(name, |end| &name[..end]);
    named_with_id(name, id_number)
}

fn signer_of(signature: &DocumentSignature) -> Option<String> {
    let id_number = without_semantics_prefix(&signature.id_number);
    let name = signature
        .name
        .strip_suffix(id_number)
        .and_then(|name| name.strip_suffix(" - "))
        .filter(|_| !id_number.is_empty())
        .unwrap_or(&signature.name);
    named_with_id(name, id_number)
}

fn named_with_id(name: &str, id: &str) -> Option<String> {
    match (name, id) {
        ("", "") => None,
        (name, "") => Some(name.to_owned()),
        ("", id) => Some(id.to_owned()),
        (name, id) => Some(format!("{name} ({id})")),
    }
}

fn in_local_time(instant: &str, time_zone: &dyn LocalTimeZone) -> String {
    match DateTime::parse_from_rfc3339(instant) {
        Ok(parsed) => {
            let utc = parsed.with_timezone(&Utc);
            utc.with_timezone(&time_zone.offset_at(utc))
                .format("%Y-%m-%d %H:%M:%S %:z")
                .to_string()
        }
        Err(_) => instant.to_owned(),
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
