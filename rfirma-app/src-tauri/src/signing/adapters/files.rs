//! El disco tras el puerto `DocumentBytes`: leer el PDF que se va a firmar.

use std::path::Path;

use crate::signing::ports::DocumentBytes;

/// Los documentos leídos del sistema de ficheros de esta máquina.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealDocumentBytes;

impl DocumentBytes for RealDocumentBytes {
    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| error.to_string())
    }
}
