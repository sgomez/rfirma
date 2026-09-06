//! Las órdenes de documentos: abrir, recientes, rúbrica, destino y el PDF firmado.

use tauri::State;

use crate::app::{self, Environment};
use crate::memory::OpenedDocuments;

use super::{
    DestinationView, Failure, OpenedDocumentView, PlacementView, RecentDocumentView,
    RubricChoiceView, RubricView, SigningSession,
};

/// Abre el diálogo del sistema y apunta lo que el portal conceda.
#[tauri::command(async)]
pub fn open_document(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<Option<OpenedDocumentView>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let configuration = environment.configuration();
    let mut dialog = app_handle.dialog().file().add_filter("PDF", &["pdf"]);
    if let Some(folder) = app::documents::starting_folder(
        &environment.memory,
        &configuration,
        &environment.documents_folder,
    ) {
        dialog = dialog.set_directory(folder);
    }
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let handle = chosen
        .into_path()
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    Ok(Some(app::documents::note_opened(
        &environment.memory,
        &configuration,
        &opened,
        handle,
    )))
}

/// Los bytes del documento abierto.
#[tauri::command(async)]
pub fn read_document(
    id: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(app::documents::bytes_of(
        &opened, &id,
    )?))
}

/// **Orden 11.** La bandeja entera, la más reciente primero.
///
/// Bandeja de documentos recientes (ADR-0010).
#[tauri::command(async)]
pub fn list_recents(
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Vec<RecentDocumentView> {
    app::recents::listed_rows(&environment.memory, &opened)
}

/// Anota en la bandeja el documento abierto y su recuadro.
#[tauri::command(async)]
pub fn record_recent(
    id: String,
    placement: Option<PlacementView>,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<RecentDocumentView, Failure> {
    app::in_hand::take(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
        placement,
    )
}

/// Quita una fila de la bandeja de recientes.
#[tauri::command(async)]
pub fn forget_recent(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<(), Failure> {
    app::recents::forget(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
    )
}

/// Abre el diálogo del portal y adopta la imagen elegida como rúbrica (ADR-0012).
#[tauri::command(async)]
pub fn choose_rubric(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Option<RubricChoiceView> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Imagen", &["png", "jpg", "jpeg"]);
    let chosen = dialog.blocking_pick_file()?;
    Some(match app::rubric::choose(&environment.rubric, chosen) {
        Ok(normalized) => RubricChoiceView::adopted(&normalized),
        Err(error) => RubricChoiceView::refused(&error),
    })
}

/// La rúbrica adoptada si la hay (ADR-0012).
#[tauri::command(async)]
pub fn read_rubric(environment: State<'_, Environment>) -> Result<Option<RubricView>, Failure> {
    let stored = app::rubric::stored(&environment.rubric)?;
    Ok(stored.map(|bytes| RubricView::from_bytes(&bytes)))
}

/// Destino previsto para el documento antes de firmar.
#[tauri::command(async)]
pub fn preview_destination(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<DestinationView, Failure> {
    let document = app::documents::opened_document(&opened, &id)?;
    Ok(app::documents::where_it_lands(
        &environment.configuration(),
        &environment.documents_folder,
        &document,
    ))
}

/// Abre el selector de directorio y guarda la carpeta de destino elegida (ADR-0011).
#[tauri::command(async)]
pub fn choose_destination(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Result<Option<String>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let Some(chosen) = app_handle.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = chosen
        .into_path()
        .map_err(|error| Failure::new("folderMissing", error.to_string()))?;
    app::configuration::choose_destination(
        &environment.memory,
        &environment.configuration,
        crate::destination::DestinationFolder::at(folder),
    )
    .map(Some)
}

/// Abre el PDF firmado con el visor del sistema (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_document(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let landing = app::signing::signed_document(&session)?;
    app_handle
        .opener()
        .open_path(landing.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// Abre la carpeta donde quedó el PDF firmado (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_folder(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let folder = app::signing::signed_folder(&session)?;
    app_handle
        .opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}
