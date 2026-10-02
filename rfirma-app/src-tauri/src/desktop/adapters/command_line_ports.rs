//! Los adaptadores de los puertos de la línea de órdenes que leen el disco y el puente nativo, no la terminal.

use std::path::Path;

use base64::Engine;

use crate::desktop::ports::{CommandLineFiles, SignatureVerifier};
use crate::signing::adapters::ffi::NativeBridge;
use crate::signing::domain::bridge::{BridgeError, Format};

/// Los ficheros del disco.
pub struct DiskFiles;

impl CommandLineFiles for DiskFiles {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| error.to_string())
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
