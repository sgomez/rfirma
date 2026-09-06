//! Caso de uso de la bandeja de documentos recientes (ADR-0010, ADR-0011).

use std::path::Path;
use std::time::SystemTime;

use crate::commands::Failure;
use crate::documents::adapters::recents_store::{Placement, RecentDocument};
use crate::documents::adapters::views::RecentDocumentView;
use crate::documents::application::opened::OpenedDocuments;
use crate::documents::domain::portal::PortalDocument;
use crate::documents::domain::recents::Badge;
use crate::signing::adapters::views::PlacementView;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::state::{BoxSize, State};
use crate::Memory;

/// Devuelve la lista de documentos recientes ordenados por fecha de uso.
pub fn listed_rows(memory: &Memory, opened: &OpenedDocuments) -> Vec<RecentDocumentView> {
    let state = loaded_state(memory);
    let size = state
        .visible_signature
        .as_ref()
        .map(|remembered| remembered.size)
        .unwrap_or_default();
    state
        .recents
        .entries()
        .iter()
        .map(|entry| told_as_row(entry, size, opened))
        .collect()
}

/// Anota un documento abierto en la bandeja de recientes y devuelve su fila para la interfaz.
pub fn record(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
    placement: Option<PlacementView>,
) -> Result<RecentDocumentView, Failure> {
    let document = opened
        .get(id)
        .ok_or_else(|| Failure::new("documentUnreadable", format!("no hay documento «{id}»")))?;
    let path = document.reading_path().to_path_buf();
    let mut state = loaded_state(memory);
    let badge = state
        .recents
        .entry(&path)
        .map_or(Badge::Unsigned, RecentDocument::badge);
    let noted = RecentDocument::seen(&path, badge, SystemTime::now())
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    let canonical = noted.path().to_path_buf();
    state.recents.record(noted);
    if let Some(placement) = placement {
        let (spot, size) = split(placement);
        state.recents.place(&canonical, Some(spot));
        remember_the_size(&mut state, size);
    }
    memory.remember_state(configuration, &state)?;
    let size = state
        .visible_signature
        .as_ref()
        .map(|remembered| remembered.size)
        .unwrap_or_default();
    let entry = state
        .recents
        .entry(&canonical)
        .expect("la fila acaba de anotarse");
    Ok(RecentDocumentView {
        id: id.to_owned(),
        ..told_as_row(entry, size, opened)
    })
}

/// Elimina un documento de la bandeja de recientes.
pub fn forget(
    memory: &Memory,
    configuration: &Configuration,
    opened: &OpenedDocuments,
    id: &str,
) -> Result<(), Failure> {
    let document = opened
        .get(id)
        .ok_or_else(|| Failure::new("documentUnreadable", format!("no hay documento «{id}»")))?;
    let mut state = loaded_state(memory);
    state
        .recents
        .forget(&canonical_or_raw(document.reading_path()));
    memory.remember_state(configuration, &state)?;
    Ok(())
}

/// Devuelve la ruta canónica o la ruta original si no puede canonicalizarse.
fn canonical_or_raw(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Anota un documento recién firmado en la bandeja con la insignia de firmado.
pub fn note_signed(memory: &Memory, configuration: &Configuration, landing: &Path) {
    let Ok(noted) = RecentDocument::seen(landing, Badge::Signed, SystemTime::now()) else {
        return;
    };
    let mut state = loaded_state(memory);
    state.recents.record(noted);
    let _ = memory.remember_state(configuration, &state);
}

/// Obtiene el estado persistido o uno por defecto si no pudo cargarse.
fn loaded_state(memory: &Memory) -> State {
    memory
        .state()
        .map(crate::signing::adapters::store::Loaded::into_value)
        .unwrap_or_default()
}

/// Convierte una entrada de recientes en una vista para la ventana.
fn told_as_row(
    entry: &RecentDocument,
    size: BoxSize,
    opened: &OpenedDocuments,
) -> RecentDocumentView {
    RecentDocumentView {
        id: identifier_for(entry.path(), opened),
        name: entry.name().to_owned(),
        badge: entry.badge(),
        modified: entry.modified(),
        last_used: entry.last_used(),
        available: entry.is_available(),
        placement: entry.placement().map(|spot| joined(spot, size)),
    }
}

/// Obtiene o asigna un identificador opaco para la ruta del documento.
fn identifier_for(path: &Path, opened: &OpenedDocuments) -> String {
    opened
        .last_id_of(path)
        .unwrap_or_else(|| opened.remember(PortalDocument::opened(path.to_path_buf())))
}

fn joined(spot: &Placement, size: BoxSize) -> PlacementView {
    PlacementView {
        pages: spot.pages.clone(),
        rect: [
            spot.lower_left_x,
            spot.lower_left_y,
            spot.lower_left_x + size.width,
            spot.lower_left_y + size.height,
        ],
    }
}

fn split(placement: PlacementView) -> (Placement, BoxSize) {
    let [x0, y0, x1, y1] = placement.rect;
    (
        Placement {
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

fn remember_the_size(state: &mut State, size: BoxSize) {
    state.visible_signature.get_or_insert_default().size = size;
}

#[cfg(test)]
mod tests;
