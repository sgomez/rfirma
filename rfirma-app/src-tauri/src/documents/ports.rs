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
