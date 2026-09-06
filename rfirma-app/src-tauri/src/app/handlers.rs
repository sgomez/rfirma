//! Casos de uso para consultar y registrar manejadores de afirma:// en el escritorio (ADR-0015).

use crate::commands::views::{UrlHandlerView, UrlHandlersView};
use crate::commands::Failure;
use crate::desktop::choice::{choose_handler_for_scheme, current_default_for_scheme};
use crate::desktop::error::Situation;
use crate::desktop::{
    registered_handlers_for_scheme, Channel, RegisteredHandlers, OUR_DESKTOP_FILE,
};
use std::path::Path;

/// Esquema de URL gestionado por la aplicación.
pub const SCHEME: &str = "afirma";

/// Consulta el estado y manejadores disponibles para el esquema afirma://.
pub fn who_handles(channel: Channel, list: &Path) -> UrlHandlersView {
    match registered_handlers_for_scheme(channel, SCHEME) {
        RegisteredHandlers::NotAvailableInsideTheSandbox => UrlHandlersView {
            available: false,
            handlers: Vec::new(),
            current: None,
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
        RegisteredHandlers::Known(handlers) => UrlHandlersView {
            available: true,
            handlers: handlers
                .iter()
                .map(|handler| UrlHandlerView {
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
pub fn chosen(channel: Channel, list: &Path, handler: &str) -> Result<(), Failure> {
    choose_handler_for_scheme(channel, list, SCHEME, handler)?;
    Ok(())
}

/// Clave del catálogo asociada a cada situación de error del escritorio (ADR-0009).
pub fn situation_name(situation: Situation) -> &'static str {
    match situation {
        Situation::NotAvailableInsideTheSandbox => "handlerNotAvailable",
        Situation::TheListIsNotReadable => "handlerListUnreadable",
        Situation::TheListIsNotWritable => "handlerListUnwritable",
    }
}

#[cfg(test)]
mod tests;
