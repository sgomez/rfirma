//! Determinación y comprobación de la carpeta de destino del documento firmado (ADR-0011).

pub use super::error::{DestinationError, Situation};
pub use super::naming::{numbered, signed_name, FIRST_NUMBER, MAX_NAMESAKES, SIGNED_SUFFIX};

use std::path::{Path, PathBuf};

use crate::documents::domain::document::Document;

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

/// Lo que la ruta es para el sistema de ficheros, sin que el dominio tenga que mirarlo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderFact {
    /// La ruta existe y es una carpeta.
    Folder,
    /// La ruta existe pero no es una carpeta.
    NotAFolder,
    /// No hay nada en la ruta.
    Missing,
    /// La ruta no se ha podido consultar.
    Unreadable(String),
}

/// Carpeta de destino cuya existencia ha sido verificada en el sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedFolder {
    path: PathBuf,
}

impl CheckedFolder {
    /// Acepta la carpeta de destino si lo que hay en su ruta lo permite (ADR-0011).
    pub fn confirmed(path: impl AsRef<Path>, fact: FolderFact) -> Result<Self, DestinationError> {
        let path = path.as_ref();
        match fact {
            FolderFact::Folder => Ok(Self {
                path: path.to_path_buf(),
            }),
            FolderFact::NotAFolder => Err(DestinationError::about(Situation::NotAFolder, path)),
            FolderFact::Missing => Err(DestinationError::about(Situation::FolderMissing, path)),
            FolderFact::Unreadable(detail) => Err(DestinationError::detailed(
                Situation::FolderUnreadable,
                path,
                detail,
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

    /// Los nombres a probar para el firmado, del preferido al último homónimo (ADR-0011).
    pub fn landing_candidates(&self, document: &Document) -> impl Iterator<Item = PathBuf> + '_ {
        let name = signed_name(document.name());
        std::iter::once(self.path.join(&name)).chain(
            (FIRST_NUMBER..=MAX_NAMESAKES)
                .map(move |number| self.path.join(numbered(&name, number))),
        )
    }

    /// El fallo cuando los mil nombres homónimos están ocupados (ADR-0011).
    pub fn no_free_name(&self, document: &Document) -> DestinationError {
        DestinationError::about(
            Situation::NoFreeName,
            &self.path.join(signed_name(document.name())),
        )
    }
}

#[cfg(test)]
mod tests;
