//! Detector puro por cabecera: «PDF / XML / binario», sin nombrar formatos de firma.

/// Lo que dice la cabecera del documento, sin más.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectedShape {
    /// Empieza por `%PDF`.
    Pdf,
    /// Prólogo `<?xml`, o primer byte no blanco `<`.
    Xml,
    /// Ni lo uno ni lo otro.
    Binary,
}

const PDF_HEADER: &[u8] = b"%PDF";

/// La forma del documento, mirando solo su cabecera.
pub fn shape_of(document: &[u8]) -> DetectedShape {
    if document.starts_with(PDF_HEADER) {
        return DetectedShape::Pdf;
    }
    match document.iter().find(|byte| !byte.is_ascii_whitespace()) {
        Some(b'<') => DetectedShape::Xml,
        _ => DetectedShape::Binary,
    }
}

#[cfg(test)]
mod tests;
