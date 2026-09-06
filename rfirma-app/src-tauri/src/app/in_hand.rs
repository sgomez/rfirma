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
mod tests;
