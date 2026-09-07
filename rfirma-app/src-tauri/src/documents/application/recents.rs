//! Caso de uso de la bandeja de documentos recientes (ADR-0010, ADR-0011).

use std::path::Path;
use std::time::SystemTime;

use crate::documents::application::documents::{self, OpenedDocuments};
use crate::documents::domain::document::Document;
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::recents::Badge;
use crate::documents::domain::recents::RecentDocument;
use crate::documents::ports::{DocumentFiles, DocumentsMemory};
use crate::signing::domain::memory_error::MemoryError;
use crate::signing::domain::BoxSize;
use crate::signing::domain::CompletedCycle;
use crate::signing::domain::Spot;
use crate::signing::domain::VisibleBox;

/// Fila de la bandeja de documentos recientes (ADR-0011).
#[derive(Clone, Debug, PartialEq)]
pub struct RecentRow {
    /// Identificador opaco del documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Insignia o estado del documento.
    pub badge: Badge,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Fecha de último uso en segundos Unix.
    pub last_used: u64,
    /// Si el fichero sigue existiendo en disco.
    pub available: bool,
    /// Recuadro guardado para este documento.
    pub placement: Option<VisibleBox>,
}

/// Por qué la bandeja no ha podido anotar u olvidar un documento.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecentsError {
    /// El documento no está abierto o no se ha podido leer.
    Document(DocumentError),
    /// El estado no se ha podido guardar (ADR-0010).
    Memory(MemoryError),
}

impl From<DocumentError> for RecentsError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}

impl From<MemoryError> for RecentsError {
    fn from(error: MemoryError) -> Self {
        Self::Memory(error)
    }
}

/// Devuelve la lista de documentos recientes ordenados por fecha de uso.
pub fn listed_rows(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
) -> Vec<RecentRow> {
    let size = memory.box_size();
    memory
        .recents()
        .entries()
        .iter()
        .map(|entry| told_as_row(files, entry, size, opened))
        .collect()
}

/// Pone delante el documento abierto y lo anota en la bandeja solo si de él queda rastro.
pub fn take(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<VisibleBox>,
) -> Result<RecentRow, RecentsError> {
    let document = documents::opened_document(opened, id)?;
    if document.is_remembered() {
        return record(memory, files, opened, id, placement);
    }
    Ok(told_without_a_row(files, id, &document, placement))
}

fn told_without_a_row(
    files: &dyn DocumentFiles,
    id: &str,
    document: &Document,
    placement: Option<VisibleBox>,
) -> RecentRow {
    RecentRow {
        id: id.to_owned(),
        name: document.name().to_owned(),
        badge: Badge::Unsigned,
        modified: documents::modified_seconds(files, document),
        last_used: now_in_seconds(),
        available: files.exists(document.reading_path()),
        placement,
    }
}

fn now_in_seconds() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Anota un documento abierto en la bandeja de recientes y devuelve su fila para la interfaz.
pub fn record(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<VisibleBox>,
) -> Result<RecentRow, RecentsError> {
    let document = opened.get(id).ok_or_else(|| no_document(id))?;
    let path = document.reading_path().to_path_buf();
    let mut recents = memory.recents();
    let badge = recents
        .entry(&path)
        .map_or(Badge::Unsigned, RecentDocument::<Spot>::badge);
    let noted = noted_now(files, &path, badge)
        .ok_or_else(|| DocumentError::Unreadable(format!("no se resuelve «{}»", path.display())))?;
    let canonical = noted.path().to_path_buf();
    recents.record(noted);
    let mut size = None;
    if let Some(placement) = placement {
        let (spot, chosen) = split(placement);
        recents.place(&canonical, Some(spot));
        size = Some(chosen);
    }
    memory.remember_recents(&recents, size)?;
    let size = memory.box_size();
    let entry = recents
        .entry(&canonical)
        .expect("la fila acaba de anotarse");
    Ok(RecentRow {
        id: id.to_owned(),
        ..told_as_row(files, entry, size, opened)
    })
}

/// Elimina un documento de la bandeja de recientes.
pub fn forget(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    opened: &OpenedDocuments,
    id: &str,
) -> Result<(), RecentsError> {
    let document = opened.get(id).ok_or_else(|| no_document(id))?;
    let mut recents = memory.recents();
    recents.forget(&canonical_or_raw(files, document.reading_path()));
    memory.remember_recents(&recents, None)?;
    Ok(())
}

fn no_document(id: &str) -> DocumentError {
    DocumentError::Unreadable(format!("no hay documento «{id}»"))
}

/// Devuelve la ruta canónica o la ruta original si no puede canonicalizarse.
fn canonical_or_raw(files: &dyn DocumentFiles, path: &Path) -> std::path::PathBuf {
    files.canonical(path).unwrap_or_else(|| path.to_path_buf())
}

fn noted_now(files: &dyn DocumentFiles, path: &Path, badge: Badge) -> Option<RecentDocument<Spot>> {
    let canonical = files.canonical(path)?;
    let modified = files.modified_seconds(&canonical);
    Some(RecentDocument::seen(
        canonical,
        modified,
        badge,
        SystemTime::now(),
    ))
}

/// Anota un documento recién firmado en la bandeja con la insignia de firmado.
pub fn note_signed(
    memory: &dyn DocumentsMemory,
    files: &dyn DocumentFiles,
    landing: &Path,
    _proof: &CompletedCycle,
) {
    let Some(noted) = noted_now(files, landing, Badge::Signed) else {
        return;
    };
    let mut recents = memory.recents();
    recents.record(noted);
    let _ = memory.remember_recents(&recents, None);
}

/// Convierte una entrada de recientes en su fila.
fn told_as_row(
    files: &dyn DocumentFiles,
    entry: &RecentDocument<Spot>,
    size: BoxSize,
    opened: &OpenedDocuments,
) -> RecentRow {
    RecentRow {
        id: identifier_for(entry.path(), opened),
        name: entry.name().to_owned(),
        badge: entry.badge(),
        modified: entry.modified(),
        last_used: entry.last_used(),
        available: files.exists(entry.path()),
        placement: entry.placement().map(|spot| joined(spot, size)),
    }
}

/// Obtiene o asigna un identificador opaco para la ruta del documento.
fn identifier_for(path: &Path, opened: &OpenedDocuments) -> String {
    opened
        .last_where(|document| document.is_remembered() && document.reading_path() == path)
        .unwrap_or_else(|| opened.mint(Document::opened(path)))
}

fn joined(spot: &Spot, size: BoxSize) -> VisibleBox {
    VisibleBox {
        pages: spot.pages.clone(),
        rect: [
            spot.lower_left_x,
            spot.lower_left_y,
            spot.lower_left_x + size.width,
            spot.lower_left_y + size.height,
        ],
    }
}

fn split(placement: VisibleBox) -> (Spot, BoxSize) {
    let [x0, y0, x1, y1] = placement.rect;
    (
        Spot {
            lower_left_x: x0,
            lower_left_y: y0,
            pages: placement.pages,
        },
        BoxSize {
            width: x1 - x0,
            height: y1 - y0,
        },
    )
}

#[cfg(test)]
mod tests;
