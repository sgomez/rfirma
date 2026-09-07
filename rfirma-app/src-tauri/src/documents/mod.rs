//! Contexto `documents` (ADR-0017): la raíz de composición y lo que presta a los vecinos.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use adapters::rubric::RubricStore;
use application::documents::OpenedDocuments;
use domain::destination::DestinationFolder;
use domain::document::Document;
use domain::error::DocumentError;
use domain::told::{DroppedDocument, SignedDocument};
use ports::{DocumentFiles, DocumentsMemory};

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
    /// El disco donde viven los documentos.
    pub files: Arc<dyn DocumentFiles + Send + Sync>,
}

impl DocumentsRoot {
    /// La carpeta de destino elegida, o la de documentos por omisión.
    pub fn chosen_folder(&self) -> DestinationFolder {
        application::documents::chosen_folder(self.memory.as_ref(), self.documents_folder.clone())
    }

    /// El documento abierto tras el asa.
    pub fn opened_document(&self, handle: &str) -> Result<Document, DocumentError> {
        application::documents::opened_document(&self.opened, handle)
    }

    /// Si del documento tras el asa se guarda rastro en la bandeja.
    pub fn is_remembered(&self, handle: &str) -> bool {
        self.opened_document(handle)
            .is_ok_and(|document| document.is_remembered())
    }

    /// Apunta un documento sin rastro y devuelve su asa.
    pub fn open_unrecorded(&self, path: PathBuf) -> String {
        self.opened.mint(Document::passing_through(path))
    }

    /// Deja el firmado en la carpeta de destino y dice dónde cayó.
    pub fn deliver(
        &self,
        document: &Document,
        signed: &[u8],
    ) -> Result<(PathBuf, SignedDocument), DocumentError> {
        application::documents::deliver(
            self.files.as_ref(),
            &self.chosen_folder(),
            document,
            signed,
        )
    }

    /// Anota el firmado en la bandeja con la prueba de que hubo un ciclo.
    pub fn note_signed(&self, landing: &Path, proof: &crate::signing::domain::CompletedCycle) {
        application::recents::note_signed(
            self.memory.as_ref(),
            self.files.as_ref(),
            landing,
            proof,
        );
    }

    /// Lo que se le cuenta a la ventana de las rutas que llegaron de fuera.
    pub fn what_was_dropped(&self, paths: &[PathBuf]) -> Option<DroppedDocument> {
        application::documents::dropped_document(self.files.as_ref(), paths, &self.opened)
    }
}
