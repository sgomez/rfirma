//! Caso de uso del documento en curso durante la sesión (ADR-0011).

use std::path::Path;

use crate::documents::application::opened::{OpenedDocuments, Remembrance};
use crate::documents::application::recents::{RecentRow, RecentsError};
use crate::documents::application::{documents, recents};
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::portal::PortalDocument;
use crate::documents::domain::recents::Badge;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::VisibleBox;
use crate::Memory;

/// Representa el documento en curso durante la sesión.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentInHand {
    id: String,
    document: PortalDocument,
    remembrance: Remembrance,
}

impl DocumentInHand {
    /// Carga el documento abierto asociado a un identificador.
    pub fn taken(opened: &OpenedDocuments, id: &str) -> Result<Self, DocumentError> {
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
    placement: Option<VisibleBox>,
) -> Result<RecentRow, RecentsError> {
    let in_hand = DocumentInHand::taken(opened, id)?;
    if in_hand.is_remembered() {
        return recents::record(memory, configuration, opened, id, placement);
    }
    Ok(told_without_a_row(&in_hand, placement))
}

/// La fila del documento en curso sin persistir en el historial.
fn told_without_a_row(in_hand: &DocumentInHand, placement: Option<VisibleBox>) -> RecentRow {
    RecentRow {
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
