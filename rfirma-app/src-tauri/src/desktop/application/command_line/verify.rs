//! La orden `verify`: valida las firmas de un fichero y deja lo que el original imprime de cada una, no el XML de `-xml`.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::{CommandLinePorts, Outcome};
use crate::desktop::domain::command_line::{
    documented, value_of, verbosity, Refusal, INPUT, JSON, XML,
};
use crate::desktop::ports::LocalTimeZone;
use crate::identity::domain::holder::without_semantics_prefix;
use crate::signing::domain::bridge::{Format, XadesVariant};
use crate::signing::domain::catalog::{counted, translated};
use crate::signing::domain::{
    DocumentFinding, DocumentSignature, DocumentSignatures, Language, SigningDate, Validity,
    ValidityReason,
};
use crate::site::domain::protocol::detection::{is_cms_signed_data, shape_of, DetectedShape};

/// Lo que el original imprime de unos datos que no son de ningún formato de firma que reconozca.
pub const UNKNOWN_FORMAT: &str = "Firma no valida: los datos proporcionados no se corresponden \
                                  con ningún formato de firma reconocido";

pub(super) fn verify(arguments: &[String], ports: &CommandLinePorts) -> Outcome {
    if let Some(parameter) = [XML, JSON]
        .into_iter()
        .find(|parameter| arguments.iter().any(|argument| argument == parameter))
    {
        return Outcome::failed(format!(
            "rfirma: el parámetro {} de «verify» todavía no está disponible en esta versión",
            documented(parameter)
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
        if verbosity(arguments) > 0 {
            return Outcome::printed(&[translated(
                ports.language,
                "panel.signed.unrecognized.title",
                &[],
            )]);
        }
        return Outcome::printed(&[UNKNOWN_FORMAT.to_owned()]);
    };
    if verbosity(arguments) > 0 {
        return with_the_signatures(&document, format, verbosity(arguments), ports);
    }
    match ports.verifier.results_of(&document, format) {
        Ok(results) => Outcome::printed(&results),
        Err(error) => Outcome::failed(format!(
            "rfirma: no se han podido validar las firmas de «{input}» ({error})"
        )),
    }
}

fn with_the_signatures(
    document: &[u8],
    format: Format,
    verbosity: usize,
    ports: &CommandLinePorts,
) -> Outcome {
    let signatures = match ports.reader.signatures_in(document) {
        Ok(signatures) => signatures,
        Err(error) => {
            let mut outcome = Outcome::printed(&[]);
            outcome.stderr.push(format!(
                "rfirma: no se han podido leer las firmas del documento: {error}"
            ));
            return outcome;
        }
    };
    let language = ports.language;
    let mut lines = vec![header_of(&signatures, format, language)];
    lines.extend(signatures.findings().iter().map(|finding| {
        format!(
            "{WARNING} {}",
            translated(language, finding_key(*finding), &[])
        )
    }));
    for (index, signature) in signatures.signatures().iter().enumerate() {
        if index == 0 || has_sheets(verbosity) {
            lines.push(String::new());
        }
        lines.extend(tree_of(signature, 0, verbosity, ports));
    }
    Outcome::printed(&lines)
}

fn has_sheets(verbosity: usize) -> bool {
    verbosity > 1
}

const VALID: &str = "✓";
const WARNING: &str = "⚠";
const INVALID: &str = "✗";

fn header_of(signatures: &DocumentSignatures, format: Format, language: Language) -> String {
    let family = family_of(format);
    if signatures.count() == 0 {
        return format!(
            "{family} · {}",
            translated(language, "commandLine.verify.noSignatures", &[])
        );
    }
    let all: Vec<&DocumentSignature> = signatures.signatures().iter().flat_map(flattened).collect();
    let counter_count = all.len() - signatures.count();
    let expired = all
        .iter()
        .filter(|signature| signature.validity == Validity::Expired)
        .count();
    let invalid = all
        .iter()
        .filter(|signature| signature.validity == Validity::Invalid)
        .count();
    let findings = signatures.findings().len();
    let mut parts = vec![
        family.to_owned(),
        counted(language, "panel.signed.count", signatures.count()),
    ];
    if counter_count > 0 {
        parts.push(counted(
            language,
            "panel.signed.countersignatureCount",
            counter_count,
        ));
    }
    if invalid + findings > 0 {
        parts.push(counted(
            language,
            "panel.previousSignatures.problems",
            expired + invalid + findings,
        ));
    } else if expired > 0 {
        parts.push(counted(
            language,
            "panel.previousSignatures.expired",
            expired,
        ));
    }
    parts.join(" · ")
}

fn flattened(signature: &DocumentSignature) -> Vec<&DocumentSignature> {
    std::iter::once(signature)
        .chain(signature.countersignatures.iter().flat_map(flattened))
        .collect()
}

fn finding_key(finding: DocumentFinding) -> &'static str {
    match finding {
        DocumentFinding::ModifiedAfterLastSignature => "documentFinding.modifiedAfterLastSignature",
        DocumentFinding::FormFilledAfterSigning => "documentFinding.formFilledAfterSigning",
        DocumentFinding::ContentAddedOnTop => "documentFinding.contentAddedOnTop",
    }
}

fn family_of(format: Format) -> &'static str {
    match format {
        Format::Xades(_) => "XAdES",
        other => other.name(),
    }
}

/// La línea de una firma y, debajo, su ficha en `-vv`; sus contrafirmas, sangradas.
fn tree_of(
    signature: &DocumentSignature,
    depth: usize,
    verbosity: usize,
    ports: &CommandLinePorts,
) -> Vec<String> {
    let indent = " ".repeat(4 * depth);
    let mut lines = vec![format!("{indent}{}", line_of(signature, ports))];
    if has_sheets(verbosity) {
        lines.extend(
            sheet_of(signature, verbosity, ports)
                .into_iter()
                .map(|line| format!("{indent}{line}")),
        );
    }
    for countersignature in &signature.countersignatures {
        if has_sheets(verbosity) {
            lines.push(String::new());
        }
        lines.extend(tree_of(countersignature, depth + 1, verbosity, ports));
    }
    lines
}

fn line_of(signature: &DocumentSignature, ports: &CommandLinePorts) -> String {
    let icon = match signature.validity {
        Validity::Valid => VALID,
        Validity::Expired => WARNING,
        Validity::Invalid => INVALID,
    };
    let (signer, on_behalf_of) = parties_of(signature);
    let mut line = format!(
        "{icon} {}",
        signer.map(|(name, _)| name).unwrap_or_default()
    );
    if let Some((entity, _)) = on_behalf_of {
        let on_behalf_of = translated(
            ports.language,
            "commandLine.verify.onBehalfOf",
            &[("entity", &entity)],
        );
        line.push_str(&format!(" · {on_behalf_of}"));
    }
    if let Some(day) = signing_instant_of(signature).map(|at| in_local_day(&at, ports.time_zone)) {
        line.push_str(&format!(" · {day}"));
    }
    line
}

fn signing_instant_of(signature: &DocumentSignature) -> Option<String> {
    match &signature.signing_date {
        Some(SigningDate::Declared { at } | SigningDate::Stamped { at, .. }) => Some(at.clone()),
        None => signature.signing_time.clone(),
    }
}

fn sheet_of(
    signature: &DocumentSignature,
    verbosity: usize,
    ports: &CommandLinePorts,
) -> Vec<String> {
    let (time_zone, language) = (ports.time_zone, ports.language);
    let (signer, on_behalf_of) = parties_of(signature);
    let issuer = Some(signature.issuer.clone()).filter(|issuer| !issuer.is_empty());
    let (date_label, date) = match &signature.signing_date {
        Some(SigningDate::Stamped { at, tsa }) => (
            "panel.signed.field.sealed",
            Some(format!("{} ({tsa})", in_local_time(at, time_zone))),
        ),
        Some(SigningDate::Declared { at }) => (
            "panel.signed.field.date",
            Some(in_local_time(at, time_zone)),
        ),
        None => (
            "panel.signed.field.date",
            signature
                .signing_time
                .as_deref()
                .map(|instant| in_local_time(instant, time_zone)),
        ),
    };
    let reason = signature
        .validity_reason
        .as_ref()
        .map(|reason| reason_text(reason, language, time_zone));
    let at_third_level =
        |value: Option<String>| value.filter(|text| verbosity > 2 && !text.is_empty());
    let serial = at_third_level(Some(signature.certificate_serial_number.clone()));
    let validity = at_third_level(certificate_validity_of(signature, language, time_zone));
    [
        ("panel.signed.field.signer", signer.map(rendered)),
        ("panel.signed.field.onBehalfOf", on_behalf_of.map(rendered)),
        ("panel.signed.field.issuer", issuer),
        (date_label, date),
        ("panel.signed.field.reason", reason),
        ("commandLine.verify.field.serialNumber", serial),
        ("commandLine.verify.field.validity", validity),
        (
            "commandLine.verify.field.algorithm",
            at_third_level(signature.signature_algorithm.clone()),
        ),
        (
            "commandLine.verify.field.profile",
            at_third_level(signature.profile.clone()),
        ),
    ]
    .into_iter()
    .filter_map(|(label, value)| value.map(|value| row(&translated(language, label, &[]), &value)))
    .collect()
}

fn row(label: &str, value: &str) -> String {
    format!("  {:<18} {value}", format!("{label}:"))
}

fn certificate_validity_of(
    signature: &DocumentSignature,
    language: Language,
    time_zone: &dyn LocalTimeZone,
) -> Option<String> {
    let from = signature
        .certificate_valid_from
        .as_deref()
        .map(|instant| in_local_time(instant, time_zone));
    let until = signature
        .certificate_valid_until
        .as_deref()
        .map(|instant| in_local_time(instant, time_zone));
    match (from, until) {
        (Some(from), Some(until)) => Some(format!("{from} – {until}")),
        (Some(from), None) => Some(translated(
            language,
            "commandLine.verify.validFrom",
            &[("date", &from)],
        )),
        (None, Some(until)) => Some(translated(
            language,
            "commandLine.verify.validUntil",
            &[("date", &until)],
        )),
        (None, None) => None,
    }
}

fn reason_text(
    reason: &ValidityReason,
    language: Language,
    time_zone: &dyn LocalTimeZone,
) -> String {
    let (key, values) = reason_key(reason, time_zone);
    let values: Vec<(&str, &str)> = values
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    translated(language, key, &values)
}

fn reason_key(
    reason: &ValidityReason,
    time_zone: &dyn LocalTimeZone,
) -> (&'static str, Vec<(&'static str, String)>) {
    match reason {
        ValidityReason::CertificateExpired { date, holder } => {
            let date = ("date", in_local_day(date, time_zone));
            match holder {
                Some(holder) => (
                    "signatureReason.certificateExpiredHolder",
                    vec![("holder", holder.clone()), date],
                ),
                None => ("signatureReason.certificateExpired", vec![date]),
            }
        }
        ValidityReason::ModifiedAfterSigning => ("signatureReason.modifiedAfterSigning", vec![]),
        ValidityReason::Damaged => ("signatureReason.damaged", vec![]),
        ValidityReason::CertificateNotYetValid { date } => (
            "signatureReason.certificateNotYetValid",
            vec![("date", in_local_day(date, time_zone))],
        ),
        ValidityReason::UnknownSignatureType => ("signatureReason.unknownSignatureType", vec![]),
        ValidityReason::CosignNotAdmitted { closed_by } => match closed_by {
            Some(closed_by) => (
                "signatureReason.cosignNotAdmitted",
                vec![("name", closed_by.clone())],
            ),
            None => ("signatureReason.cosignNotAdmittedUnnamed", vec![]),
        },
    }
}

type Party = (String, String);

fn rendered((name, id): Party) -> String {
    named_with_id(&name, &id).unwrap_or_default()
}

fn parties_of(signature: &DocumentSignature) -> (Option<Party>, Option<Party>) {
    let Some(identifier) = signature
        .organization_identifier
        .as_deref()
        .map(without_semantics_prefix)
    else {
        return (Some(signer_of(signature)), None);
    };
    let entity = || {
        (
            signature.organization_name.clone().unwrap_or_default(),
            identifier.to_owned(),
        )
    };
    let id_number = without_semantics_prefix(&signature.id_number);
    if id_number.is_empty() || id_number == identifier {
        (Some(entity()), None)
    } else {
        (Some(representative_of(signature)), Some(entity()))
    }
}

fn representative_of(signature: &DocumentSignature) -> Party {
    let id_number = without_semantics_prefix(&signature.id_number);
    let name = signature.name.as_str();
    let name = name.strip_prefix(id_number).map_or(name, str::trim_start);
    let name = name.rfind(" (R: ").map_or(name, |end| &name[..end]);
    (name.to_owned(), id_number.to_owned())
}

fn signer_of(signature: &DocumentSignature) -> Party {
    let id_number = without_semantics_prefix(&signature.id_number);
    let name = signature
        .name
        .strip_suffix(id_number)
        .and_then(|name| name.strip_suffix(" - "))
        .filter(|_| !id_number.is_empty())
        .unwrap_or(&signature.name);
    (name.to_owned(), id_number.to_owned())
}

fn named_with_id(name: &str, id: &str) -> Option<String> {
    match (name, id) {
        ("", "") => None,
        (name, "") => Some(name.to_owned()),
        ("", id) => Some(id.to_owned()),
        (name, id) => Some(format!("{name} ({id})")),
    }
}

fn in_local_day(instant: &str, time_zone: &dyn LocalTimeZone) -> String {
    in_local_time(instant, time_zone)
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_owned()
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
