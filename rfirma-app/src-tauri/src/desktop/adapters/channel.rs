//! Canal de distribución y consulta de manejadores del esquema afirma:// en el escritorio (ADR-0015).

use std::path::Path;

/// Fichero testigo que indica ejecución dentro de un contenedor flatpak.
const SANDBOX_MARKER: &str = "/.flatpak-info";

pub use crate::desktop::domain::channel::Channel;

impl Channel {
    /// Detecta el canal examinando la presencia del testigo de sandbox.
    pub fn detected() -> Self {
        Self::over(Path::new(SANDBOX_MARKER))
    }

    /// Determina el canal según la existencia de la ruta testigo.
    fn over(marker: &Path) -> Self {
        if marker.exists() {
            Self::Flatpak
        } else {
            Self::Native
        }
    }
}

/// Manejadores registrados según las capacidades del entorno.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisteredHandlers {
    /// Lista de manejadores proporcionada por el escritorio.
    Known(Vec<RegisteredHandler>),
    /// No disponible dentro del sandbox flatpak.
    NotAvailableInsideTheSandbox,
}

/// Consulta los manejadores registrados en el escritorio para un esquema.
pub fn registered_handlers_for_scheme(channel: Channel, scheme: &str) -> RegisteredHandlers {
    match channel {
        Channel::Flatpak => RegisteredHandlers::NotAvailableInsideTheSandbox,
        Channel::Native => {
            #[cfg(target_os = "linux")]
            let handlers = gio::AppInfo::all_for_type(&content_type_for(scheme))
                .iter()
                .filter_map(|info| {
                    let id = gio::prelude::AppInfoExt::id(info)?;
                    Some(RegisteredHandler::new(
                        gio::prelude::AppInfoExt::name(info).to_string(),
                        id.to_string(),
                    ))
                })
                .collect();
            #[cfg(windows)]
            let handlers = handlers_in_the_windows_registry_pending(scheme);
            RegisteredHandlers::Known(handlers)
        }
    }
}

/// Los manejadores del registro de Windows, que aún no se consultan: ninguno.
#[cfg(windows)]
fn handlers_in_the_windows_registry_pending(_scheme: &str) -> Vec<RegisteredHandler> {
    Vec::new()
}

/// Manejador registrado con nombre visible e identificador de escritorio.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredHandler {
    name: String,
    id: String,
}

impl RegisteredHandler {
    /// Construye un manejador registrado.
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
        }
    }

    /// Nombre visible del manejador para la interfaz.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Identificador del fichero .desktop asociado.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// Tipo MIME asociado al esquema de URL.
pub(crate) fn content_type_for(scheme: &str) -> String {
    format!("x-scheme-handler/{scheme}")
}

#[cfg(test)]
mod tests;
