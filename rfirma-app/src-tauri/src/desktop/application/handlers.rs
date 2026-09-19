//! Casos de uso para consultar y registrar manejadores de afirma:// en el escritorio (ADR-0015).

use crate::desktop::domain::error::DesktopError;
use crate::desktop::domain::handlers::{UrlHandlers, OUR_DESKTOP_FILE};
use crate::desktop::domain::withdrawal::Withdrawal;
use crate::desktop::ports::HandlerRegistry;

/// Esquema de URL gestionado por la aplicación.
pub const SCHEME: &str = "afirma";

/// Consulta el estado y manejadores disponibles para el esquema afirma://.
pub fn who_handles(registry: &dyn HandlerRegistry) -> UrlHandlers {
    match registry.registered_for(SCHEME) {
        None => UrlHandlers {
            available: false,
            handlers: Vec::new(),
            current: None,
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
        Some(handlers) => UrlHandlers {
            available: true,
            handlers,
            current: registry.current_default_for(SCHEME),
            ours: OUR_DESKTOP_FILE.to_owned(),
        },
    }
}

/// Registra un manejador como predeterminado para afirma:// en mimeapps.list.
pub fn chosen(registry: &dyn HandlerRegistry, handler: &str) -> Result<(), DesktopError> {
    registry.choose_for(SCHEME, handler)
}

/// Retira rFirma como manejador de afirma://, sin registrar ningún otro en su lugar (ID-364).
pub fn withdrawn(registry: &dyn HandlerRegistry) -> Withdrawal {
    let was_ours = registry.current_default_for(SCHEME).as_deref() == Some(OUR_DESKTOP_FILE);
    match registry.remove_for(SCHEME) {
        Ok(()) if was_ours => Withdrawal::Withdrawn,
        Ok(()) => Withdrawal::WasNotThere,
        Err(error) => Withdrawal::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
