//! Puertos del contexto de documentos: la memoria entre sesiones (ADR-0010) y el disco donde viven.

use std::path::{Path, PathBuf};

use crate::documents::domain::destination::{DestinationFolder, FolderFact};
use crate::documents::domain::recents::Recents;
use crate::memory_error::MemoryError;
use crate::signing::domain::{BoxSize, Spot};

/// La memoria vista desde los documentos: el destino elegido, la última carpeta abierta y la bandeja.
pub trait DocumentsMemory {
    /// La carpeta de destino que la persona eligió, si eligió alguna.
    fn chosen_destination(&self) -> Option<DestinationFolder>;

    /// La última carpeta desde la que se abrió un documento.
    fn last_open_folder(&self) -> Option<PathBuf>;

    /// Apunta la carpeta desde la que se acaba de abrir un documento.
    fn remember_last_open_folder(&self, folder: &Path) -> Result<(), MemoryError>;

    /// La bandeja de recientes guardada, o vacía.
    fn recents(&self) -> Recents<Spot>;

    /// El tamaño global del recuadro que la bandeja recuerda.
    fn box_size(&self) -> BoxSize;

    /// Guarda la bandeja, y el tamaño del recuadro si cambió, según permitan los interruptores.
    fn remember_recents(
        &self,
        recents: &Recents<Spot>,
        size: Option<BoxSize>,
    ) -> Result<(), MemoryError>;
}

/// El disco visto desde los documentos: lo que hace falta para abrirlos, entregarlos y recordarlos.
pub trait DocumentFiles {
    /// Qué es la ruta para el sistema de ficheros.
    fn folder_fact(&self, path: &Path) -> FolderFact;

    /// Si ya hay algo en esa ruta.
    fn exists(&self, path: &Path) -> bool;

    /// Si el fichero se deja abrir para lectura.
    fn readable(&self, path: &Path) -> Result<(), String>;

    /// El contenido del fichero.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;

    /// Escribe el contenido en la ruta indicada.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;

    /// Instante de última modificación en segundos desde la época UNIX.
    fn modified_seconds(&self, path: &Path) -> Option<u64>;

    /// La ruta canónica, o nada si no se puede resolver.
    fn canonical(&self, path: &Path) -> Option<PathBuf>;

    /// Los ficheros dentro de la carpeta, ordenados; vacío si no es una carpeta legible.
    fn files_within(&self, folder: &Path) -> Vec<PathBuf>;
}

/// Pistas para los diálogos de selección y guardado del portal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DialogClues {
    /// Título de la ventana de diálogo.
    pub title: Option<String>,
    /// Nombre de fichero sugerido o propuesto.
    pub filename: Option<String>,
    /// Extensiones admitidas por el filtro (ej. "pdf", "p12").
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones (ej. "Documento PDF").
    pub description: Option<String>,
    /// Directorio inicial sugerido para abrir el diálogo.
    pub starting_folder: Option<PathBuf>,
}

impl DialogClues {
    /// Pistas vacías por omisión.
    pub fn new() -> Self {
        Self::default()
    }

    /// Añade el título del diálogo.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Añade el nombre de fichero propuesto.
    pub fn with_filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Añade un filtro de extensiones con su descripción.
    pub fn with_filter(
        mut self,
        description: impl Into<String>,
        extensions: &[impl AsRef<str>],
    ) -> Self {
        self.description = Some(description.into());
        self.extensions = extensions
            .iter()
            .map(|ext| ext.as_ref().to_owned())
            .collect();
        self
    }

    /// Añade el directorio inicial.
    pub fn with_starting_folder(mut self, folder: impl Into<PathBuf>) -> Self {
        self.starting_folder = Some(folder.into());
        self
    }
}

/// Puerto de interacción con los diálogos del sistema a través del portal.
pub trait PortalDialogs: Send + Sync {
    /// Abre el diálogo para seleccionar un único fichero.
    fn pick_file(&self, clues: &DialogClues) -> Result<Option<PathBuf>, String>;

    /// Abre el diálogo para seleccionar uno o varios ficheros.
    fn pick_files(&self, clues: &DialogClues) -> Result<Vec<PathBuf>, String>;

    /// Abre el diálogo para guardar un fichero con las pistas dadas.
    fn save_file(&self, clues: &DialogClues) -> Result<Option<PathBuf>, String>;
}

#[cfg(test)]
mod tests;
