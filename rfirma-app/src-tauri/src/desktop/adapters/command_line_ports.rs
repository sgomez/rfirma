//! Los adaptadores de los puertos de la línea de órdenes que leen el disco y el puente nativo, no la terminal.

use std::path::Path;

use base64::Engine;
use chrono::{DateTime, FixedOffset, Local, Offset, Utc};

use crate::desktop::ports::{
    CertificateFilter, CommandLineFiles, LocalTimeZone, SignatureReader, SignatureVerifier,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::adapters::ffi::NativeBridge;
use crate::signing::domain::bridge::{BridgeError, Format};
use crate::signing::domain::DocumentSignatures;
use crate::site::domain::protocol::SiteFilter;
use crate::site::ports::FilterEngine;

/// Los ficheros del disco.
pub struct DiskFiles;

impl CommandLineFiles for DiskFiles {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| error.to_string())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|error| error.to_string())
    }
}

/// El validador del original en la librería nativa, que se carga solo cuando se le pregunta.
pub struct NativeVerifier;

impl SignatureVerifier for NativeVerifier {
    fn results_of(&self, document: &[u8], format: Format) -> Result<Vec<String>, BridgeError> {
        let document_b64 = base64::engine::general_purpose::STANDARD.encode(document);
        NativeBridge::open()?.verify_signatures(&document_b64, format)
    }
}

/// La lectura de firmas de la raíz de `signing`, que carga el puente solo cuando se le pregunta.
pub struct NativeReader;

impl SignatureReader for NativeReader {
    fn signatures_in(&self, document: &[u8]) -> Result<DocumentSignatures, BridgeError> {
        crate::signing::signatures_in(document)
    }
}

/// La zona horaria del sistema, la que dice `TZ` si está puesta.
pub struct SystemTimeZone;

impl LocalTimeZone for SystemTimeZone {
    fn offset_at(&self, instant: DateTime<Utc>) -> FixedOffset {
        instant.with_timezone(&Local).offset().fix()
    }
}

/// El motor de filtros del original en la librería nativa, que se carga solo cuando se le pregunta.
pub struct NativeFilter;

impl CertificateFilter for NativeFilter {
    fn accepted(
        &self,
        filter: &SiteFilter,
        certificates: Vec<TokenCertificate>,
    ) -> Result<Vec<TokenCertificate>, String> {
        let certificates: Vec<TokenCertificate> = certificates
            .into_iter()
            .filter(|certificate| certificate.status().is_usable())
            .collect();
        if certificates.is_empty() {
            return Ok(certificates);
        }
        let payload = certificates
            .iter()
            .map(|certificate| base64::engine::general_purpose::STANDARD.encode(certificate.der()))
            .collect::<Vec<_>>()
            .join(";");
        let engine = NativeBridge::open().map_err(|error| error.to_string())?;
        let indexes = engine
            .select(&filter.as_java_properties(), &payload)
            .map_err(|error| error.to_string())?;
        Ok(certificates
            .into_iter()
            .enumerate()
            .filter(|(index, _)| indexes.contains(index))
            .map(|(_, certificate)| certificate)
            .collect())
    }
}
