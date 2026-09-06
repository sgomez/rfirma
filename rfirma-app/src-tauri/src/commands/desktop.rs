//! Las órdenes del escritorio: invocación, versión publicada y manejadores afirma://.

use tauri::State;

use crate::app::{self, Environment};
use crate::memory::OpenedDocuments;

use super::{DroppedDocumentView, Failure, NewVersionView, PendingInvocation, UrlHandlersView};

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    pending: State<'_, PendingInvocation>,
    opened: State<'_, OpenedDocuments>,
) -> Option<DroppedDocumentView> {
    let invocation = pending.take()?;
    app::invocation::invoked_document(&invocation, &opened)
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(environment: State<'_, Environment>) -> Option<NewVersionView> {
    let announced = app::version::new_version(
        app::version::Version::running(),
        &environment.memory,
        &crate::releases::latest_release,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.to_string(),
    })
}

/// Manejadores registrados para el esquema afirma:// en el escritorio (ADR-0015).
#[tauri::command(async)]
pub fn url_handlers() -> UrlHandlersView {
    let channel = crate::desktop::Channel::detected();
    let list = crate::desktop::choice::mimeapps_list_from_environment().unwrap_or_default();
    app::handlers::who_handles(channel, &list)
}

/// Establece el manejador preferido para el esquema afirma:// (ADR-0015).
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::Channel::detected();
    let list = crate::desktop::choice::mimeapps_list_from_environment().map_err(|error| {
        Failure::new(
            app::handlers::situation_name(crate::desktop::error::Situation::TheListIsNotWritable),
            error.to_string(),
        )
    })?;
    app::handlers::chosen(channel, &list, &handler)
}
