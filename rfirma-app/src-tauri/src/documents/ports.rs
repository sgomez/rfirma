//! Puertos del contexto de documentos: lo que la memoria entre sesiones guarda de ellos (ADR-0010).

use std::path::{Path, PathBuf};

use crate::documents::domain::destination::DestinationFolder;
use crate::documents::domain::recents::Recents;
use crate::signing::domain::memory_error::MemoryError;
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
