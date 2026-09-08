//! El formato que la sede nombra en `format=`, cerrado y con los alias de `AOSignConstants`.

use super::detection::{shape_of, DetectedShape};

/// Cómo envuelve la sede una firma XAdES.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XadesEnvelope {
    /// `XAdES Detached`.
    Detached,
    /// `XAdES Enveloping`.
    Enveloping,
    /// `XAdES Enveloped`.
    Enveloped,
    /// `XAdES-ASiC-S`.
    AsicS,
}

/// El formato de firma que pide la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestedFormat {
    /// Firma PAdES sobre un PDF.
    Pades,
    /// Firma CAdES.
    Cades,
    /// Firma CAdES en un contenedor ASiC-S.
    CadesAsicS,
    /// Firma CMS / PKCS#7.
    Cms,
    /// Firma XAdES en una de sus envolturas.
    Xades(XadesEnvelope),
    /// Firma de una factura electrónica.
    FacturaE,
}

/// Cada nombre de `AOSignConstants` que rFirma lee, con el formato que nombra.
const NAMED: [(&str, RequestedFormat); 16] = [
    ("pades", RequestedFormat::Pades),
    ("padestri", RequestedFormat::Pades),
    ("adobe pdf", RequestedFormat::Pades),
    ("cades", RequestedFormat::Cades),
    ("cadestri", RequestedFormat::Cades),
    ("cades-asic-s", RequestedFormat::CadesAsicS),
    ("cms/pkcs#7", RequestedFormat::Cms),
    ("xades", RequestedFormat::Xades(XadesEnvelope::Enveloping)),
    (
        "xadestri",
        RequestedFormat::Xades(XadesEnvelope::Enveloping),
    ),
    (
        "xades enveloping",
        RequestedFormat::Xades(XadesEnvelope::Enveloping),
    ),
    (
        "xades detached",
        RequestedFormat::Xades(XadesEnvelope::Detached),
    ),
    (
        "xades enveloped",
        RequestedFormat::Xades(XadesEnvelope::Enveloped),
    ),
    ("xades-asic-s", RequestedFormat::Xades(XadesEnvelope::AsicS)),
    ("facturae", RequestedFormat::FacturaE),
    ("facturaetri", RequestedFormat::FacturaE),
    ("factura-e", RequestedFormat::FacturaE),
];

impl RequestedFormat {
    /// El formato que nombra ese `format=`, o nada si el original no lo firma trifásico.
    pub fn named(text: &str) -> Option<Self> {
        let asked = text.trim().to_ascii_lowercase();
        NAMED
            .iter()
            .find(|(name, _)| *name == asked)
            .map(|(_, format)| *format)
    }
}

/// El formato efectivo de `format=auto`, leído de la cabecera del documento.
pub fn format_of(document: &[u8]) -> RequestedFormat {
    match shape_of(document) {
        DetectedShape::Pdf => RequestedFormat::Pades,
        DetectedShape::Invoice => RequestedFormat::FacturaE,
        DetectedShape::Xml => RequestedFormat::Xades(XadesEnvelope::Enveloping),
        DetectedShape::Binary => RequestedFormat::Cades,
    }
}

#[cfg(test)]
mod tests;
