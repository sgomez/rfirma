//! Las órdenes del escritorio: invocación, versión publicada y manejadores afirma://.

use tauri::State;

use crate::documents::application::opened::OpenedDocuments;
use crate::Environment;

use super::views::{NewVersionView, UrlHandlersView};
use crate::commands::Failure;
use crate::desktop::application::invocation::PendingInvocation;
use crate::documents::adapters::views::DroppedDocumentView;

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    pending: State<'_, PendingInvocation>,
    opened: State<'_, OpenedDocuments>,
) -> Option<DroppedDocumentView> {
    let invocation = pending.take()?;
    crate::desktop::application::invocation::invoked_document(&invocation, &opened)
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(environment: State<'_, Environment>) -> Option<NewVersionView> {
    let announced = crate::desktop::application::version::new_version(
        crate::desktop::application::version::Version::running(),
        &environment.memory,
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
    crate::desktop::application::handlers::who_handles(channel, &list)
}

/// Establece el manejador preferido para el esquema afirma:// (ADR-0015).
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list =
        crate::desktop::adapters::choice::mimeapps_list_from_environment().map_err(|error| {
            Failure::new(
                crate::desktop::application::handlers::situation_name(
                    crate::desktop::domain::error::Situation::TheListIsNotWritable,
                ),
                error.to_string(),
            )
        })?;
    crate::desktop::application::handlers::chosen(channel, &list, &handler)
}
