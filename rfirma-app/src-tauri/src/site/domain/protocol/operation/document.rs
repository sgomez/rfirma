//! La lectura del `dat`: descarga, Base64 a la manera del original y gzip.

use super::super::codes::{Parameter, SafCode};
use super::super::data_source::{download_url, DataSource};
use super::super::refusal::Refusal;
use super::super::url::AfirmaUrl;
use super::properties::required;

/// El `dat` de una firma, si vino: ausente no es rechazo, vacío sí lo es.
pub(super) fn optional_document(
    url: &AfirmaUrl,
    data: &dyn DataSource,
) -> Result<Option<Vec<u8>>, Refusal> {
    if url.parameter("dat").is_none() {
        return Ok(None);
    }
    Ok(Some(read_document(url, data)?))
}

/// El documento de `dat`, ya obtenido: se lee una sola vez, la pida quien lo pida.
pub(super) fn read_document(url: &AfirmaUrl, data: &dyn DataSource) -> Result<Vec<u8>, Refusal> {
    let document = data_of(url, data)?;
    if document.is_empty() {
        return Err(Refusal::new(
            SafCode::SignWithoutData,
            "el parametro 'dat' viene vacio: no hay nada que firmar",
        ));
    }
    Ok(document)
}

/// Los bytes de `dat`: bajados de su URL, descodificados del Base64 o, si no lo es, su texto
/// tal cual (`DataDownloader.downloadData`, 1.9.2).
pub(super) fn data_of(url: &AfirmaUrl, data: &dyn DataSource) -> Result<Vec<u8>, Refusal> {
    let value = required(url, "dat", Parameter::Data)?;
    if is_gzip(url) && is_base64_to_the_original(value) {
        let compressed = decode_like_the_original(value)
            .ok_or_else(|| Refusal::about(Parameter::Data, "el parametro 'dat' no es Base64"))?;
        if compressed.is_empty() {
            return Ok(compressed);
        }
        return decompress_gzip(&compressed);
    }

    let trimmed = java_trim(value);
    if let Some(remote) = download_url(trimmed) {
        return data.download(remote).map_err(|detail| {
            Refusal::about(
                Parameter::Data,
                format!("no se han podido obtener los datos de '{remote}': {detail}"),
            )
        });
    }
    if trimmed.starts_with(FTP) {
        return Err(Refusal::about(
            Parameter::Data,
            format!("rFirma no baja datos por ftp: '{trimmed}'"),
        ));
    }

    if is_base64_to_the_original(trimmed) {
        if let Some(decoded) = decode_like_the_original(trimmed) {
            return Ok(decoded);
        }
    }
    Ok(trimmed.as_bytes().to_vec())
}

const FTP: &str = "ftp://";

/// El alfabeto de `Base64.isBase64` (1.9.2), con sus espacios y su `~`.
const ALPHABET_OF_THE_ORIGINAL: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz=_-\t\n+/0123456789\r~";

/// Si `Base64.isBase64` (1.9.2) lo daría por Base64: su alfabeto, el `=` solo al final y una
/// longitud múltiplo de cuatro sin contar los saltos de línea.
pub(super) fn is_base64_to_the_original(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut counted = 0;
    for (position, byte) in bytes.iter().enumerate() {
        if !ALPHABET_OF_THE_ORIGINAL.contains(byte) {
            return false;
        }
        if *byte == b'=' && position + 2 < bytes.len() {
            return false;
        }
        if *byte != b'\n' && *byte != b'\r' {
            counted += 1;
        }
    }
    counted % 4 == 0
}

/// El Base64 como lo lee `Base64.decode` (1.9.2): salta los espacios, descarta el cuarteto
/// incompleto del final y no mira los bits sobrantes; nada si hay un carácter que no descodifica.
fn decode_like_the_original(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return Some(Vec::new());
    }
    if bytes.len() < 4 {
        return None;
    }
    let mut decoded = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut quartet = [(0_u8, 0_u32); 4];
    let mut filled = 0;
    for &byte in bytes {
        let sextet = match byte {
            b' ' | b'\t' | b'\n' | b'\r' => continue,
            b'=' => 0xFF,
            other => sextet_of(other)?,
        };
        quartet[filled] = (byte, sextet);
        filled += 1;
        if filled == 4 {
            decode_quartet(&quartet, &mut decoded);
            filled = 0;
            if byte == b'=' {
                break;
            }
        }
    }
    Some(decoded)
}

fn sextet_of(byte: u8) -> Option<u32> {
    let sextet = match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' | b'-' => 62,
        b'/' | b'_' => 63,
        _ => return None,
    };
    Some(u32::from(sextet))
}

fn decode_quartet(quartet: &[(u8, u32); 4], decoded: &mut Vec<u8>) {
    let [(_, first), (_, second), (third_byte, third), (fourth_byte, fourth)] = *quartet;
    let bits = first << 18 | second << 12;
    if third_byte == b'=' {
        decoded.push((bits >> 16) as u8);
        return;
    }
    let bits = bits | third << 6;
    if fourth_byte == b'=' {
        decoded.extend([(bits >> 16) as u8, (bits >> 8) as u8]);
        return;
    }
    let bits = bits | fourth;
    decoded.extend([(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]);
}

/// `String.trim` de Java: fuera todo carácter hasta el espacio por los dos extremos.
fn java_trim(value: &str) -> &str {
    value.trim_matches(|character: char| character <= ' ')
}

pub(super) fn is_gzip(url: &AfirmaUrl) -> bool {
    url.parameter("gzip")
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn decompress_gzip(compressed: &[u8]) -> Result<Vec<u8>, Refusal> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|error| {
        Refusal::about(
            Parameter::Data,
            format!("el parametro 'dat' no es un gzip valido: {error}"),
        )
    })?;
    Ok(decompressed)
}
