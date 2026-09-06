//! Casos de uso para consultar y registrar manejadores de afirma:// en el escritorio (ADR-0015).

use crate::desktop::adapters::channel::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers, OUR_DESKTOP_FILE,
};
use crate::desktop::adapters::choice::{choose_handler_for_scheme, current_default_for_scheme};
use crate::desktop::domain::error::DesktopError;
use crate::desktop::domain::handlers::{UrlHandler, UrlHandlers};
use std::path::Path;

/// Esquema de URL gestionado por la aplicación.
pub const SCHEME: &str = "afirma";

/// Consulta el estado y manejadores disponibles para el esquema afirma://.
pub fn who_handles(channel: Channel, list: &Path) -> UrlHandlers {
    match registered_handlers_for_scheme(channel, SCHEME) {
        RegisteredHandlers::NotAvailableInsideTheSandbox => UrlHandlers {
            available: false,
            handlers: Vec::new(),
            current: None,
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
        RegisteredHandlers::Known(handlers) => UrlHandlers {
            available: true,
            handlers: handlers
                .iter()
                .map(|handler| UrlHandler {
                    id: handler.id().to_owned(),
                    name: handler.name().to_owned(),
                })
                .collect(),
            current: current_default_for_scheme(channel, list, SCHEME),
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
    }
}

/// Registra un manejador como predeterminado para afirma:// en mimeapps.list.
pub fn chosen(channel: Channel, list: &Path, handler: &str) -> Result<(), DesktopError> {
    choose_handler_for_scheme(channel, list, SCHEME, handler)?;
    Ok(())
}

#[cfg(test)]
mod tests;
