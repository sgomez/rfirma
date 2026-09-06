//! Caso de uso del documento en curso durante la sesión (ADR-0011).

use std::path::Path;

use crate::app::{documents, recents};
use crate::commands::views::{Failure, PlacementView, RecentDocumentView};
use crate::destination::PortalDocument;
use crate::memory::{Badge, Configuration, Memory, OpenedDocuments, Remembrance};

/// Representa el documento en curso durante la sesión.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInHand {
    id: String,
    document: PortalDocument,
    remembrance: Remembrance,
}

impl DocumentInHand {
    /// Carga el documento abierto asociado a un identificador.
    pub fn taken(opened: &OpenedDocuments, id: &str) -> Result<Self, Failure> {
        let document = documents::opened_document(opened, id)?;
        let remembrance = opened.remembrance(id).unwrap_or(Remembrance::Unrecorded);
        Ok(Self {
            id: id.to_owned(),
            document,
            remembrance,
        })
    }

    /// Identificador del documento.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Documento gestionado por el portal.
    pub fn document(&self) -> &PortalDocument {
        &self.document
    }

    /// Por dónde se leen sus bytes. **No cruza a la ventana** (ADR-0011).
    pub fn reading_path(&self) -> &Path {
        self.document.reading_path()
    }

    /// Indica si el documento debe registrarse en el historial.
    pub fn is_remembered(&self) -> bool {
        self.remembrance == Remembrance::Remembered
    }
}

/// Pone delante el documento abierto y lo registra en la bandeja si corresponde.
pub fn take(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<PlacementView>,
) -> Result<RecentDocumentView, Failure> {
    let in_hand = DocumentInHand::taken(opened, id)?;
    if in_hand.is_remembered() {
        return recents::record(memory, configuration, opened, id, placement);
    }
    Ok(told_without_a_row(&in_hand, placement))
}

/// Construye una vista del documento en curso sin persistir en el historial.
fn told_without_a_row(
    in_hand: &DocumentInHand,
    placement: Option<PlacementView>,
) -> RecentDocumentView {
    RecentDocumentView {
        id: in_hand.id().to_owned(),
        name: in_hand.document().name().to_owned(),
        badge: Badge::Unsigned,
        modified: documents::modified_seconds(in_hand.document()),
        last_used: now_in_seconds(),
        available: in_hand.reading_path().exists(),
        placement,
    }
}

fn now_in_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fixtures::a_memory;
    use crate::signing::PageSet;

    fn a_pdf(directory: &Path, name: &str) -> std::path::PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, b"%PDF-1.7\n").expect("deberia escribirse");
        path
    }

    fn a_placement() -> PlacementView {
        PlacementView {
            rect: [10.0, 20.0, 210.0, 70.0],
            pages: PageSet::only_page(3),
        }
    }

    #[test]
    fn a_document_that_is_remembered_still_leaves_its_row() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "contrato.pdf");
        let id = opened.remember(PortalDocument::opened(path));

        let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
            .expect("deberia ponerse delante");

        assert_eq!(row.name, "contrato.pdf");
        assert_eq!(row.placement, Some(a_placement()));
        assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
    }

    #[test]
    fn a_document_that_is_not_remembered_leaves_neither_row_nor_placement() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "de-la-sede.pdf");
        let id = opened.remember_unrecorded(PortalDocument::opened(path));

        let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
            .expect("deberia ponerse delante");

        assert_eq!(row.id, id);
        assert_eq!(row.name, "de-la-sede.pdf");
        assert!(recents::listed_rows(&memory, &opened).is_empty());
        let remembered = memory
            .state()
            .map(crate::memory::Loaded::into_value)
            .ok()
            .and_then(|state| state.visible_signature);
        assert_eq!(
            remembered, None,
            "el tamano del recuadro tampoco se recuerda"
        );
    }

    #[test]
    fn remembrance_belongs_to_the_grant_and_not_to_the_file() {
        let home = tempfile::tempdir().expect("deberia crearse");
        let memory = a_memory(home.path());
        let configuration = Configuration::default();
        let opened = OpenedDocuments::new();
        let path = a_pdf(home.path(), "contrato.pdf");
        let unrecorded = opened.remember_unrecorded(PortalDocument::opened(path.clone()));
        let remembered = opened.remember(PortalDocument::opened(path));

        take(&memory, &configuration, &opened, &unrecorded, None).expect("deberia ponerse delante");
        assert!(recents::listed_rows(&memory, &opened).is_empty());

        take(&memory, &configuration, &opened, &remembered, None).expect("deberia ponerse delante");
        assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
    }

    #[test]
    fn an_identifier_of_no_session_puts_nothing_in_hand() {
        let opened = OpenedDocuments::new();

        let taken = DocumentInHand::taken(&opened, "00000000000000000000000000000000");

        assert!(taken.is_err());
    }
}
