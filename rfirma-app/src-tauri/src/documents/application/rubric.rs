//! Caso de uso para la adopción y consulta de la rúbrica (ADR-0012).

use tauri_plugin_dialog::FilePath;

use crate::documents::adapters::rubric::RubricStore;
use crate::documents::domain::rubric::{NormalizedRubric, RubricError, Situation};

/// Adopta la imagen seleccionada por el usuario en el almacén de rúbricas.
pub fn choose(store: &RubricStore, chosen: FilePath) -> Result<NormalizedRubric, RubricError> {
    let source = chosen
        .into_path()
        .map_err(|error| RubricError::new(Situation::SourceUnreadable, error.to_string()))?;
    store.adopt(&source)
}

/// Consulta la rúbrica guardada en el almacén si existe.
pub fn stored(store: &RubricStore) -> Result<Option<Vec<u8>>, RubricError> {
    store.stored()
}

#[cfg(test)]
mod tests;
