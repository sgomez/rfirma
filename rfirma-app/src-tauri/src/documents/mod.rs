//! Contexto `documents` (ADR-0017): la raíz de composición y lo que presta a los vecinos.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapters::rubric::RubricStore;
use application::opened::OpenedDocuments;
use domain::destination::DestinationFolder;
use domain::dropped::Dropped;
use domain::error::DocumentError;
use domain::portal::PortalDocument;
use domain::told::{DroppedDocument, SignedDocument};
use ports::DocumentsMemory;

/// La raíz de `documents`: la carpeta por omisión, la rúbrica, lo abierto en esta sesión y la memoria.
pub struct DocumentsRoot {
    /// Carpeta de documentos del usuario por omisión.
    pub documents_folder: PathBuf,
    /// Almacén de la rúbrica (ADR-0012).
    pub rubric: RubricStore,
    /// Los documentos abiertos en esta sesión.
    pub opened: OpenedDocuments,
    /// Lo que la memoria entre sesiones guarda de los documentos.
    pub memory: Arc<dyn DocumentsMemory + Send + Sync>,
}

impl DocumentsRoot {
    /// La carpeta de destino elegida, o la de documentos por omisión.
    pub fn chosen_folder(&self) -> DestinationFolder {
        application::documents::chosen_folder(self.memory.as_ref(), self.documents_folder.clone())
    }

    /// El documento abierto tras el asa.
    pub fn opened_document(&self, handle: &str) -> Result<PortalDocument, DocumentError> {
        application::documents::opened_document(&self.opened, handle)
    }

    /// Si del documento tras el asa se guarda rastro en la bandeja.
    pub fn is_remembered(&self, handle: &str) -> bool {
        application::in_hand::DocumentInHand::taken(&self.opened, handle)
            .is_ok_and(|in_hand| in_hand.is_remembered())
    }

    /// Apunta un documento sin rastro y devuelve su asa.
    pub fn open_unrecorded(&self, path: PathBuf) -> String {
        self.opened
            .remember_unrecorded(PortalDocument::opened(path))
    }

    /// Deja el firmado en la carpeta de destino y dice dónde cayó.
    pub fn deliver(
        &self,
        document: &PortalDocument,
        signed: &[u8],
    ) -> Result<(PathBuf, SignedDocument), DocumentError> {
        application::documents::deliver(&self.chosen_folder(), document, signed)
    }

    /// Anota el firmado en la bandeja con la prueba de que hubo un ciclo.
    pub fn note_signed(&self, landing: &Path, proof: &crate::signing::domain::CompletedCycle) {
        application::recents::note_signed(self.memory.as_ref(), landing, proof);
    }

    /// Lo que se le cuenta a la ventana de lo que llegó de fuera.
    pub fn told_as_dropped(&self, dropped: Dropped) -> Option<DroppedDocument> {
        application::documents::told_as_dropped(dropped, &self.opened)
    }
}
