//! Los adaptadores de los puertos de la línea de órdenes que leen el disco, la zona horaria y el puente nativo —validador, filtros y motor de firmas—, no la terminal.

use std::path::Path;

use base64::Engine;
use chrono::{DateTime, FixedOffset, Local, Offset, Utc};

use crate::desktop::ports::{
    CertificateFilter, CommandLineFiles, LocalTimeZone, SignatureReading, SignatureVerifier,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::adapters::ffi::NativeBridge;
use crate::signing::domain::bridge::{BridgeError, Format};
use crate::signing::domain::DocumentSignatures;
use crate::signing::ports::PreviousSignaturesEngine;
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

/// El motor de firmas previas en la librería nativa, que se carga solo cuando se le pregunta.
pub struct NativeEngine;

impl PreviousSignaturesEngine for NativeEngine {
    fn previous_signatures(&self, document_b64: &str) -> Result<DocumentSignatures, BridgeError> {
        NativeBridge::open()?.previous_signatures(document_b64)
    }
}

/// La lectura de firmas de la raíz de `signing` sobre el motor que se le da.
pub struct EngineReading<'a>(&'a dyn PreviousSignaturesEngine);

impl<'a> EngineReading<'a> {
    /// Lee con este motor.
    pub fn over(engine: &'a dyn PreviousSignaturesEngine) -> Self {
        Self(engine)
    }
}

impl SignatureReading for EngineReading<'_> {
    fn signatures_in(&self, document: &[u8]) -> Result<DocumentSignatures, String> {
        crate::signing::read_signatures(document, self.0).map_err(|error| error.to_string())
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
