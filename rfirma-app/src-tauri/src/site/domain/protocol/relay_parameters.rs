//! El XML de parámetros que la sede sube al servlet de almacenamiento cuando la operación no cabe en la URL.

use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use quick_xml::XmlVersion;

use super::refusal::Refusal;
use super::url::{url_decode, AfirmaUrl};

/// El nombre de cada entrada del XML, calcado del original.
const ENTRY: &[u8] = b"e";

/// El parámetro que nombra el verbo, y el nombre de raíz que el original entiende como ausente.
const OPERATION: &str = "op";

/// El verbo que el original asume cuando la raíz se llama como el propio parámetro.
const DEFAULT_OPERATION: &str = "sign";

/// La operación que describe el XML de parámetros (`ProtocolInvocationUriParserUtil.parseXml`).
pub fn operation_of_the_parameters_xml(xml: &[u8]) -> Result<AfirmaUrl, Refusal> {
    let text = std::str::from_utf8(xml)
        .map_err(|error| Refusal::params(format!("el XML de parametros no es UTF-8: {error}")))?;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut root: Option<String> = None;
    let mut pairs: Vec<(String, String)> = Vec::new();

    loop {
        let event = reader.read_event().map_err(|error| {
            Refusal::params(format!("el XML de parametros no se puede leer: {error}"))
        })?;

        match event {
            Event::Start(tag) | Event::Empty(tag) => match root {
                None => root = Some(name_of(&tag)),
                Some(_) if tag.local_name().as_ref() == ENTRY => pairs.push(entry_of(&tag)?),
                Some(_) => return Err(malformed()),
            },
            Event::Text(_) | Event::CData(_) => return Err(malformed()),
            Event::Eof => break,
            _ => {}
        }
    }

    let Some(root) = root else {
        return Err(Refusal::params(
            "el XML de parametros del servidor intermedio no trae ningun elemento",
        ));
    };

    Ok(AfirmaUrl::of(&verb_of(&root), pairs))
}

/// El verbo que nombra la raíz: `sign` cuando se llama como el propio parámetro `op`.
fn verb_of(root: &str) -> String {
    if root.eq_ignore_ascii_case(OPERATION) {
        return DEFAULT_OPERATION.to_owned();
    }
    root.to_owned()
}

fn name_of(tag: &BytesStart) -> String {
    String::from_utf8_lossy(tag.local_name().as_ref()).into_owned()
}

fn entry_of(tag: &BytesStart) -> Result<(String, String), Refusal> {
    match (attribute_of(tag, b"k"), attribute_of(tag, b"v")) {
        (Some(key), Some(value)) => Ok((key, url_decode(&value))),
        _ => Err(malformed()),
    }
}

fn attribute_of(tag: &BytesStart, name: &[u8]) -> Option<String> {
    tag.attributes()
        .flatten()
        .find(|attribute: &Attribute<'_>| attribute.key.as_ref() == name)
        .and_then(|attribute| attribute.normalized_value(XmlVersion::Implicit1_0).ok())
        .map(|value| value.into_owned())
}

fn malformed() -> Refusal {
    Refusal::params("el XML de parametros del servidor intermedio no tiene la forma esperada")
}

#[cfg(test)]
mod tests;
