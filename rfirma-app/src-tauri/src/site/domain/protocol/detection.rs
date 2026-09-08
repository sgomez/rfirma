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

const PDF_HEADER: &[u8] = b"%PDF";

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
