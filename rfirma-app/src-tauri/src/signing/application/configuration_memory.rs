//! Estructura de configuración persistida entre sesiones (ADR-0010).

use serde::{Deserialize, Serialize};

use crate::documents::domain::destination::DestinationFolder;
use crate::signing::domain::Language;

/// El tema de la ventana: lo que el usuario elige ver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Lo que diga el sistema operativo.
    #[default]
    System,
    /// Claro, pase lo que pase.
    Light,
    /// Oscuro, pase lo que pase.
    Dark,
}

/// Configuración del usuario persistida en disco (ADR-0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    /// Idioma de la interfaz y del texto de la firma visible (ADR-0010).
    pub language: Language,
    /// Dónde cae el documento firmado.
    pub destination: Option<DestinationFolder>,
    /// Indica si se recuerda la última configuración de firma visible.
    pub remember_visible_signature: bool,
    /// Indica si se recuerdan los documentos recientes y el certificado.
    pub remember_activity: bool,
    /// Indica si se debe notificar cuando haya una versión nueva.
    pub notify_new_version: bool,
    /// El tema de la ventana.
    pub theme: Theme,
    /// Indica si el asistente del primer arranque ya se ha visto.
    pub setup_wizard_seen: bool,
    /// Indica si el botón de consentir de la ventana de sede espera una cuenta atrás.
    pub consent_countdown: bool,
    /// Indica si la sede puede elegir sola el único certificado candidato (ADR-0032).
    pub honour_automatic_selection: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            destination: None,
            remember_visible_signature: true,
            remember_activity: true,
            notify_new_version: true,
            theme: Theme::System,
            setup_wizard_seen: false,
            consent_countdown: true,
            honour_automatic_selection: false,
        }
    }
}

#[cfg(test)]
mod tests;
