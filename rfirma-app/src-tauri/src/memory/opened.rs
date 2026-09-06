//! Registro en memoria de documentos abiertos en la sesión activa (ADR-0011).

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::handles::mint;
use crate::destination::PortalDocument;

/// Modalidad de persistencia asociada a un documento abierto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remembrance {
    /// Documento local que genera historial y recuerda estado.
    Remembered,
    /// Documento efímero o de sede del que no se guarda rastro.
    Unrecorded,
}

/// Colección en memoria de los documentos abiertos en la sesión actual.
#[derive(Debug, Default)]
pub struct OpenedDocuments {
    documents: Mutex<HashMap<String, Grant>>,
    granted: AtomicU64,
}

#[derive(Debug)]
struct Grant {
    order: u64,
    document: PortalDocument,
    remembrance: Remembrance,
}

impl OpenedDocuments {
    /// Construye una colección vacía de documentos abiertos.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un documento abierto recordando su historial (ADR-0011).
    pub fn remember(&self, document: PortalDocument) -> String {
        self.grant(document, Remembrance::Remembered)
    }

    /// Registra un documento abierto sin guardar rastro en el historial (ADR-0011).
    pub fn remember_unrecorded(&self, document: PortalDocument) -> String {
        self.grant(document, Remembrance::Unrecorded)
    }

    fn grant(&self, document: PortalDocument, remembrance: Remembrance) -> String {
        let id = mint();
        let order = self.granted.fetch_add(1, Ordering::Relaxed);
        lock(&self.documents).insert(
            id.clone(),
            Grant {
                order,
                document,
                remembrance,
            },
        );
        id
    }

    /// Obtiene el documento asociado al identificador si existe.
    pub fn get(&self, id: &str) -> Option<PortalDocument> {
        lock(&self.documents)
            .get(id)
            .map(|grant| grant.document.clone())
    }

    /// Obtiene la modalidad de persistencia del documento si existe.
    pub fn remembrance(&self, id: &str) -> Option<Remembrance> {
        lock(&self.documents).get(id).map(|grant| grant.remembrance)
    }

    /// Obtiene el identificador más reciente asociado a una ruta de lectura.
    pub fn last_id_of(&self, reading_path: &Path) -> Option<String> {
        lock(&self.documents)
            .iter()
            .filter(|(_, grant)| grant.remembrance == Remembrance::Remembered)
            .filter(|(_, grant)| grant.document.reading_path() == reading_path)
            .max_by_key(|(_, grant)| grant.order)
            .map(|(id, _)| id.clone())
    }

    /// Número de documentos actualmente registrados.
    pub fn len(&self) -> usize {
        lock(&self.documents).len()
    }

    /// Comprueba si no hay documentos registrados.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
