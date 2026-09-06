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
mod tests {
    use super::*;

    const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

    #[test]
    fn an_opened_document_comes_back_by_its_identifier() {
        let opened = OpenedDocuments::new();

        let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.get(&id),
            Some(PortalDocument::opened(A_PORTAL_HANDLE))
        );
    }

    #[test]
    fn an_identifier_nobody_minted_is_simply_not_there() {
        let opened = OpenedDocuments::new();

        assert_eq!(opened.get("00000000000000000000000000000000"), None);
        assert!(opened.is_empty());
    }

    #[test]
    fn documents_opened_one_after_another_all_stay_open() {
        let opened = OpenedDocuments::new();

        let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let second = opened.remember(PortalDocument::opened(
            "/run/user/1000/doc/2f9c94ca/factura.pdf",
        ));

        assert_ne!(first, second);
        assert_eq!(opened.len(), 2);
        assert_eq!(
            opened
                .get(&first)
                .map(|document| document.name().to_owned()),
            Some("contrato.pdf".to_owned())
        );
        assert_eq!(
            opened
                .get(&second)
                .map(|document| document.name().to_owned()),
            Some("factura.pdf".to_owned())
        );
    }

    #[test]
    fn a_grant_says_whether_the_document_it_stands_for_is_remembered() {
        let opened = OpenedDocuments::new();

        let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let unrecorded = opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.remembrance(&remembered),
            Some(Remembrance::Remembered)
        );
        assert_eq!(
            opened.remembrance(&unrecorded),
            Some(Remembrance::Unrecorded)
        );
        assert_eq!(opened.remembrance("00000000000000000000000000000000"), None);
    }

    #[test]
    fn the_tray_never_borrows_the_identifier_of_a_document_that_is_not_remembered() {
        let opened = OpenedDocuments::new();
        let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(
            opened.last_id_of(Path::new(A_PORTAL_HANDLE)),
            Some(remembered)
        );
    }

    #[test]
    fn the_identifier_carries_nothing_of_the_path_it_stands_for() {
        let opened = OpenedDocuments::new();

        let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
        for leak in ["/", "run", "doc", "1e8b83b9", "contrato", "pdf"] {
            assert!(
                !id.contains(leak),
                "el identificador «{id}» lleva «{leak}» dentro"
            );
        }
    }

    #[test]
    fn the_same_document_opened_twice_is_minted_twice() {
        let opened = OpenedDocuments::new();

        let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
        let second = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

        assert_ne!(first, second);
    }
}
