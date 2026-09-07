//! Las órdenes de documentos: abrir, recientes, rúbrica, destino y el PDF firmado.

use tauri::State;

use crate::documents::DocumentsRoot;
use crate::signing::SigningRoot;

use super::tauri_rubric::{RubricChoiceView, RubricView};
use super::views::{DestinationView, OpenedDocumentView, RecentDocumentView};
use crate::commands::Failure;
use crate::documents::domain::rubric::{RubricError, Situation};
use crate::signing::adapters::views::PlacementView;
use crate::signing::domain::VisibleBox;

/// Abre el diálogo del sistema y apunta lo que el portal conceda.
#[tauri::command(async)]
pub fn open_document(
    app_handle: tauri::AppHandle,
    documents: State<'_, DocumentsRoot>,
) -> Result<Option<OpenedDocumentView>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let mut dialog = app_handle.dialog().file().add_filter("PDF", &["pdf"]);
    if let Some(folder) = crate::documents::application::documents::starting_folder(
        documents.memory.as_ref(),
        &documents.chosen_folder(),
    ) {
        dialog = dialog.set_directory(folder);
    }
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let handle = chosen
        .into_path()
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    Ok(Some(
        crate::documents::application::documents::note_opened(
            documents.memory.as_ref(),
            &documents.opened,
            handle,
        )
        .into(),
    ))
}

/// Los bytes del documento abierto.
#[tauri::command(async)]
pub fn read_document(
    id: String,
    documents: State<'_, DocumentsRoot>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(
        crate::documents::application::documents::bytes_of(&documents.opened, &id)?,
    ))
}

/// **Orden 11.** La bandeja entera, la más reciente primero.
///
/// Bandeja de documentos recientes (ADR-0010).
#[tauri::command(async)]
pub fn list_recents(documents: State<'_, DocumentsRoot>) -> Vec<RecentDocumentView> {
    crate::documents::application::recents::listed_rows(
        documents.memory.as_ref(),
        &documents.opened,
    )
    .into_iter()
    .map(RecentDocumentView::from)
    .collect()
}

/// Anota en la bandeja el documento abierto y su recuadro.
#[tauri::command(async)]
pub fn record_recent(
    id: String,
    placement: Option<PlacementView>,
    documents: State<'_, DocumentsRoot>,
) -> Result<RecentDocumentView, Failure> {
    Ok(crate::documents::application::recents::take(
        documents.memory.as_ref(),
        &documents.opened,
        &id,
        placement.map(VisibleBox::from),
    )?
    .into())
}

/// Quita una fila de la bandeja de recientes.
#[tauri::command(async)]
pub fn forget_recent(id: String, documents: State<'_, DocumentsRoot>) -> Result<(), Failure> {
    Ok(crate::documents::application::recents::forget(
        documents.memory.as_ref(),
        &documents.opened,
        &id,
    )?)
}

/// Abre el diálogo del portal y adopta la imagen elegida como rúbrica (ADR-0012).
#[tauri::command(async)]
pub fn choose_rubric(
    app_handle: tauri::AppHandle,
    documents: State<'_, DocumentsRoot>,
) -> Option<RubricChoiceView> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Imagen", &["png", "jpg", "jpeg"]);
    let chosen = dialog.blocking_pick_file()?;
    let adopted = chosen
        .into_path()
        .map_err(|error| RubricError::new(Situation::SourceUnreadable, error.to_string()))
        .and_then(|source| documents.rubric.adopt(&source));
    Some(match adopted {
        Ok(normalized) => RubricChoiceView::adopted(&normalized),
        Err(error) => RubricChoiceView::refused(&error),
    })
}

/// La rúbrica adoptada si la hay (ADR-0012).
#[tauri::command(async)]
pub fn read_rubric(documents: State<'_, DocumentsRoot>) -> Result<Option<RubricView>, Failure> {
    let stored = documents.rubric.stored()?;
    Ok(stored.map(|bytes| RubricView::from_bytes(&bytes)))
}

/// Destino previsto para el documento antes de firmar.
#[tauri::command(async)]
pub fn preview_destination(
    id: String,
    documents: State<'_, DocumentsRoot>,
) -> Result<DestinationView, Failure> {
    let document = documents.opened_document(&id)?;
    Ok(crate::documents::application::documents::where_it_lands(
        &documents.chosen_folder(),
        &document,
    )
    .into())
}

/// Abre el selector de directorio y guarda la carpeta de destino elegida (ADR-0011).
#[tauri::command(async)]
pub fn choose_destination(
    app_handle: tauri::AppHandle,
    signing: State<'_, SigningRoot>,
) -> Result<Option<String>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let Some(chosen) = app_handle.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = chosen
        .into_path()
        .map_err(|error| Failure::new("folderMissing", error.to_string()))?;
    let (next, name) = crate::signing::application::configuration::with_destination(
        &signing.configuration(),
        crate::documents::domain::destination::DestinationFolder::at(folder),
    );
    signing.memory.remember_configuration(&next)?;
    Ok(Some(name))
}

/// Abre el PDF firmado con el visor del sistema (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_document(
    app_handle: tauri::AppHandle,
    signing: State<'_, SigningRoot>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let landing = signing.signed_document()?;
    app_handle
        .opener()
        .open_path(landing.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// Abre la carpeta donde quedó el PDF firmado (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_folder(
    app_handle: tauri::AppHandle,
    signing: State<'_, SigningRoot>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let folder = signing.signed_folder()?;
    app_handle
        .opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}
