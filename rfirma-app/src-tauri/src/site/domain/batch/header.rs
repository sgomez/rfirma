//! El algoritmo de la cabecera del lote remoto, que se lee al firmar sus `PK1` y no al analizar la petición (`BatchSigner`, 1.9.2).

use super::BatchFormat;
use crate::site::domain::protocol::{AlgorithmReading, AskedAlgorithm};

/// Por qué la cabecera del lote no da un algoritmo que rFirma firme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderRefusal {
    /// El lote pide SHA-1 (ADR-0023).
    Sha1(String),
    /// La cabecera falta, no se lee o nombra un algoritmo que rFirma no firma.
    Unreadable(String),
}

/// El `algorithm` del lote, ya admitido: atributo de `<signbatch>` en el XML heredado, o campo del objeto raíz en JSON.
pub fn batch_algorithm(format: BatchFormat, lote: &[u8]) -> Result<String, HeaderRefusal> {
    let algorithm = match format {
        BatchFormat::Json => algorithm_in_json(lote),
        BatchFormat::Xml => algorithm_in_xml(lote),
    }
    .map_err(HeaderRefusal::Unreadable)?;
    match AskedAlgorithm::read(&algorithm) {
        AlgorithmReading::Attended(_) => Ok(algorithm),
        AlgorithmReading::Sha1 => Err(HeaderRefusal::Sha1(sha1_detail(&algorithm))),
        AlgorithmReading::Unrecognized => Err(HeaderRefusal::Unreadable(format!(
            "el algoritmo de lote '{algorithm}' no se atiende"
        ))),
    }
}

/// El detalle con el que se rechaza un lote que pide SHA-1.
pub fn sha1_detail(algorithm: &str) -> String {
    format!("el algoritmo '{algorithm}' es SHA-1: rFirma firma con SHA-2")
}

fn algorithm_in_json(lote: &[u8]) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_slice(lote)
        .map_err(|error| format!("el lote no es JSON valido: {error}"))?;
    value
        .get("algorithm")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "falta el parametro 'algorithm'".to_owned())
}

fn algorithm_in_xml(lote: &[u8]) -> Result<String, String> {
    let text =
        std::str::from_utf8(lote).map_err(|error| format!("el lote no es UTF-8: {error}"))?;
    let mut reader = quick_xml::Reader::from_str(text);
    loop {
        match reader.read_event() {
            Ok(quick_xml::events::Event::Start(tag) | quick_xml::events::Event::Empty(tag)) => {
                return tag
                    .attributes()
                    .flatten()
                    .find(|attribute| attribute.key.as_ref() == b"algorithm")
                    .map(|attribute| String::from_utf8_lossy(attribute.value.as_ref()).into_owned())
                    .ok_or_else(|| "falta el parametro 'algorithm'".to_owned());
            }
            Ok(quick_xml::events::Event::Eof) => {
                return Err("el lote no tiene elemento raiz".to_owned())
            }
            Err(error) => return Err(format!("el lote no es XML valido: {error}")),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
