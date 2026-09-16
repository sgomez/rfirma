//! Detector puro por cabecera: «PDF / factura / XML / binario», sin nombrar formatos de firma.

use quick_xml::events::Event;
use quick_xml::Reader;

/// Lo que dice la cabecera del documento, sin más.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectedShape {
    /// Empieza por `%PDF`.
    Pdf,
    /// XML cuya raíz es una factura electrónica.
    Invoice,
    /// Prólogo `<?xml`, o primer byte no blanco `<`.
    Xml,
    /// Ni lo uno ni lo otro.
    Binary,
}

/// Estructura de firma previa detectable en el documento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectedSignature {
    /// PDF con diccionario de firma.
    Pdf,
    /// Factura electrónica.
    Invoice,
    /// XML con elemento Signature.
    Xml,
    /// Contenedor CMS/CAdES SignedData.
    Cms,
}

const PDF_HEADER: &[u8] = b"%PDF";
const BYTE_RANGE: &[u8] = b"/ByteRange";
const TYPE_SIG: &[u8] = b"/Type /Sig";
const TYPE_SIG_NO_SPACE: &[u8] = b"/Type/Sig";
const SIGNATURE_TAG: &[u8] = b"Signature";

const OID_SIGNED_DATA: &[u8] = &[
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02,
];

/// El nombre local de la raíz de una factura electrónica.
const INVOICE_ROOT: &[u8] = b"Facturae";

/// Los tres hijos que el original le exige a esa raíz.
const INVOICE_CHILDREN: [&[u8]; 3] = [b"FileHeader", b"Parties", b"Invoices"];

/// La forma del documento, mirando solo su cabecera.
pub fn shape_of(document: &[u8]) -> DetectedShape {
    if document.starts_with(PDF_HEADER) {
        return DetectedShape::Pdf;
    }
    match document.iter().find(|byte| !byte.is_ascii_whitespace()) {
        Some(b'<') if is_an_invoice(document) => DetectedShape::Invoice,
        Some(b'<') => DetectedShape::Xml,
        _ => DetectedShape::Binary,
    }
}

/// Comprueba si los datos contienen una estructura de firma previa reconocible.
pub fn detect_signature(document: &[u8]) -> Option<DetectedSignature> {
    if document.starts_with(PDF_HEADER) && has_pdf_signatures(document) {
        return Some(DetectedSignature::Pdf);
    }
    if is_an_invoice(document) {
        return Some(DetectedSignature::Invoice);
    }
    if is_cms_signed_data(document) {
        return Some(DetectedSignature::Cms);
    }
    if document
        .iter()
        .any(|b| !b.is_ascii_whitespace() && *b == b'<')
        && has_xml_signatures(document)
    {
        return Some(DetectedSignature::Xml);
    }
    None
}

/// Comprueba si un PDF contiene una estructura de firma previa.
pub fn has_pdf_signatures(document: &[u8]) -> bool {
    document.starts_with(PDF_HEADER)
        && (contains_subsequence(document, BYTE_RANGE)
            || contains_subsequence(document, TYPE_SIG)
            || contains_subsequence(document, TYPE_SIG_NO_SPACE))
}

/// Comprueba si un documento XML contiene un elemento de firma electrónica.
pub fn has_xml_signatures(document: &[u8]) -> bool {
    let mut reader = Reader::from_reader(document);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(tag)) | Ok(Event::Empty(tag)) => {
                if tag.local_name().as_ref() == SIGNATURE_TAG {
                    return true;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    false
}

/// Comprueba si los datos corresponden a un contenedor CMS/CAdES SignedData.
pub fn is_cms_signed_data(document: &[u8]) -> bool {
    if document.len() < 14 || document[0] != 0x30 {
        return false;
    }
    let mut idx = 1;
    let first_len = document[idx];
    if first_len <= 0x80 {
        idx += 1;
    } else {
        let num_bytes = (first_len & 0x7f) as usize;
        if num_bytes == 0 || num_bytes > 4 || idx + 1 + num_bytes > document.len() {
            return false;
        }
        idx += 1 + num_bytes;
    }
    if idx + OID_SIGNED_DATA.len() > document.len() {
        return false;
    }
    document[idx..idx + OID_SIGNED_DATA.len()] == *OID_SIGNED_DATA
}

fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// La misma comprobación que `AOFacturaESigner.isValidDataFile`: la raíz y sus tres hijos.
fn is_an_invoice(document: &[u8]) -> bool {
    let mut reader = Reader::from_reader(document);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut depth = 0_usize;
    let mut pending = INVOICE_CHILDREN.to_vec();
    loop {
        let opened = match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(tag)) => Some((tag.local_name().as_ref().to_vec(), true)),
            Ok(Event::Empty(tag)) => Some((tag.local_name().as_ref().to_vec(), false)),
            Ok(Event::End(_)) => {
                depth = depth.saturating_sub(1);
                None
            }
            Ok(Event::Eof) => break,
            Ok(_) => None,
            Err(_) => return false,
        };
        if let Some((name, nested)) = opened {
            if depth == 0 && name != INVOICE_ROOT {
                return false;
            }
            if depth == 1 {
                pending.retain(|child| *child != name.as_slice());
            }
            if nested {
                depth += 1;
            }
        }
        buffer.clear();
    }
    pending.is_empty()
}

#[cfg(test)]
mod tests;
