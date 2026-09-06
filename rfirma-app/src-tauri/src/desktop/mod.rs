//! Canal de distribución y consulta de manejadores del esquema afirma:// en el escritorio (ADR-0015).

pub mod choice;
pub mod error;

use std::path::Path;

/// Fichero testigo que indica ejecución dentro de un contenedor flatpak.
const SANDBOX_MARKER: &str = "/.flatpak-info";

/// Fichero .desktop con el que rFirma queda registrada en paquetes nativos.
pub const OUR_DESKTOP_FILE: &str = "rfirma.desktop";

/// Canal de distribución en el que corre el proceso (ADR-0015).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// Instalación nativa sin aislamiento (.deb o .rpm).
    Native,
    /// Instalación en contenedor flatpak.
    Flatpak,
}

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
            RegisteredHandlers::Known(handlers)
        }
    }
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
fn content_type_for(scheme: &str) -> String {
    format!("x-scheme-handler/{scheme}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_marker_present_means_the_flatpak_channel() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let marker = directory.path().join(".flatpak-info");
        fs::write(&marker, b"[Application]\n").expect("deberia escribirse");

        assert_eq!(Channel::over(&marker), Channel::Flatpak);
    }

    #[test]
    fn no_marker_means_the_native_channel() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        assert_eq!(
            Channel::over(&directory.path().join(".flatpak-info")),
            Channel::Native
        );
    }

    #[test]
    fn the_real_question_is_asked_over_the_well_known_marker() {
        assert_eq!(SANDBOX_MARKER, "/.flatpak-info");
    }

    #[test]
    fn inside_the_sandbox_there_is_no_answer_at_all() {
        let handlers = registered_handlers_for_scheme(Channel::Flatpak, "afirma");

        assert_eq!(handlers, RegisteredHandlers::NotAvailableInsideTheSandbox);
    }

    #[test]
    fn outside_the_sandbox_the_answer_comes_from_the_desktop() {
        let handlers = registered_handlers_for_scheme(Channel::Native, "afirma");

        assert!(matches!(handlers, RegisteredHandlers::Known(_)));
    }

    #[test]
    fn our_desktop_file_is_the_one_the_bundler_installs() {
        let configuration: serde_json::Value =
            serde_json::from_str(include_str!("../../tauri.conf.json"))
                .expect("tauri.conf.json deberia ser JSON");
        let product = configuration["productName"]
            .as_str()
            .expect("tauri.conf.json deberia declarar productName");

        assert_eq!(OUR_DESKTOP_FILE, format!("{product}.desktop"));
    }

    #[test]
    fn a_handler_carries_both_the_name_and_the_desktop_file() {
        let handler = RegisteredHandler::new("AutoFirma", "autofirma.desktop");

        assert_eq!(handler.name(), "AutoFirma");
        assert_eq!(handler.id(), "autofirma.desktop");
    }

    #[test]
    fn every_offered_handler_can_be_written_as_a_default() {
        let RegisteredHandlers::Known(handlers) =
            registered_handlers_for_scheme(Channel::Native, "http")
        else {
            panic!("fuera del sandbox tiene que haber lista");
        };

        assert!(handlers.iter().all(|handler| !handler.id().is_empty()));
    }
}
