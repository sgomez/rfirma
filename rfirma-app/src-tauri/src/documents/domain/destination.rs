//! Determinación y comprobación de la carpeta de destino del documento firmado (ADR-0011).

pub use super::error::{DestinationError, Situation};
pub use super::naming::{numbered, signed_name, FIRST_NUMBER, MAX_NAMESAKES, SIGNED_SUFFIX};

use std::path::{Path, PathBuf};

use crate::documents::domain::portal::PortalDocument;

use serde::{Deserialize, Serialize};

/// Carpeta configurada para guardar los documentos firmados.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationFolder {
    path: PathBuf,
}

impl DestinationFolder {
    /// Construye una carpeta de destino con la ruta indicada.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Ruta de la carpeta de destino.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre del segmento final de la carpeta para visualización.
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }
}

/// Carpeta de destino cuya existencia ha sido verificada en el sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFolder {
    path: PathBuf,
}

impl CheckedFolder {
    /// Comprueba la existencia y validez de la carpeta de destino (ADR-0011).
    pub fn check(folder: &DestinationFolder) -> Result<Self, DestinationError> {
        Self::at(folder.path())
    }

    /// Comprueba la existencia y validez de una ruta de destino.
    pub fn at(path: impl AsRef<Path>) -> Result<Self, DestinationError> {
        let path = path.as_ref();
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(Self {
                path: path.to_path_buf(),
            }),
            Ok(_) => Err(DestinationError::about(Situation::NotAFolder, path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(DestinationError::about(Situation::FolderMissing, path))
            }
            Err(error) => Err(DestinationError::caused_by(
                Situation::FolderUnreadable,
                path,
                &error,
            )),
        }
    }

    /// Ruta verificada de la carpeta de destino.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Nombre del segmento final de la carpeta para visualización (ADR-0011).
    pub fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    }

    /// Calcula la ruta final del documento firmado resolviendo homónimos (ADR-0011).
    pub fn landing_for(&self, document: &PortalDocument) -> Result<PathBuf, DestinationError> {
        let name = signed_name(document.name());
        let first = self.path.join(&name);
        if !first.exists() {
            return Ok(first);
        }
        for number in FIRST_NUMBER..=MAX_NAMESAKES {
            let candidate = self.path.join(numbered(&name, number));
            if !candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(DestinationError::about(Situation::NoFreeName, &first))
    }
}

#[cfg(test)]
mod tests;
