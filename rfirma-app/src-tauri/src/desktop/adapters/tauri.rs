//! Las órdenes del escritorio: invocación, versión publicada y manejadores afirma://.

use tauri::State;

use crate::desktop::DesktopRoot;
use crate::documents::DocumentsRoot;

use super::registry::DesktopRegistry;
use super::views::{NewVersionView, UrlHandlersView};
use crate::crossing::Failure;
use crate::desktop::domain::error::{DesktopError, Situation};
use crate::documents::adapters::views::DroppedDocumentView;

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    desktop: State<'_, DesktopRoot>,
    documents: State<'_, DocumentsRoot>,
) -> Option<DroppedDocumentView> {
    let invocation = desktop.pending_invocation.take()?;
    let paths = crate::desktop::application::invocation::invoked_documents(&invocation)?;
    documents
        .what_was_dropped(&paths)
        .map(DroppedDocumentView::from)
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(desktop: State<'_, DesktopRoot>) -> Option<NewVersionView> {
    let announced = crate::desktop::application::version::new_version(
        crate::desktop::application::version::Version::running(),
        desktop.memory.as_ref(),
        &crate::desktop::adapters::releases::latest_release,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.to_string(),
    })
}

/// Manejadores registrados para el esquema afirma:// en el escritorio (ADR-0015).
#[tauri::command(async)]
pub fn url_handlers() -> UrlHandlersView {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list =
        crate::desktop::adapters::choice::mimeapps_list_from_environment().unwrap_or_default();
    crate::desktop::application::handlers::who_handles(&DesktopRegistry::of(channel, list)).into()
}

/// Establece el manejador preferido para el esquema afirma:// (ADR-0015).
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list = crate::desktop::adapters::choice::mimeapps_list_from_environment()
        .map_err(|error| DesktopError::new(Situation::TheListIsNotWritable, error.to_string()))?;
    Ok(crate::desktop::application::handlers::chosen(
        &DesktopRegistry::of(channel, list),
        &handler,
    )?)
}
