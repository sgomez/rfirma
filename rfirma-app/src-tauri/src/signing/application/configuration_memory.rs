//! Estructura de configuración persistida entre sesiones (ADR-0010).

use serde::{Deserialize, Deserializer, Serialize};

use crate::documents::domain::destination::{DestinationFolder, DestinationMode};
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

/// Versión actual del asistente del primer arranque; subirla lo vuelve a mostrar.
pub const SETUP_WIZARD_VERSION: u32 = 2;

/// Configuración del usuario persistida en disco (ADR-0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    /// Idioma de la interfaz y del texto de la firma visible (ADR-0010).
    pub language: Language,
    /// Dónde cae el documento firmado.
    pub destination: Option<DestinationFolder>,
    /// Si el firmado cae junto al original o en la carpeta de destino.
    pub destination_mode: DestinationMode,
    /// Indica si se recuerda la última configuración de firma visible.
    pub remember_visible_signature: bool,
    /// Indica si se recuerdan los documentos recientes y el certificado.
    pub remember_activity: bool,
    /// Indica si se debe notificar cuando haya una versión nueva.
    pub notify_new_version: bool,
    /// El tema de la ventana.
    pub theme: Theme,
    /// Versión del asistente del primer arranque vista por última vez; 0 si ninguna.
    #[serde(alias = "setup_wizard_seen", deserialize_with = "seen_version")]
    pub setup_wizard_version_seen: u32,
    /// Indica si el botón de consentir de la ventana de sede espera una cuenta atrás.
    pub consent_countdown: bool,
    /// Indica si la sede puede elegir sola el único certificado candidato (ADR-0032).
    pub honour_automatic_selection: bool,
    /// Indica si se permite firmar con SHA-1 fuera de XML.
    pub allow_sha1: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            destination: None,
            destination_mode: DestinationMode::NextToTheOriginal,
            remember_visible_signature: true,
            remember_activity: true,
            notify_new_version: true,
            theme: Theme::System,
            setup_wizard_version_seen: 0,
            consent_countdown: true,
            honour_automatic_selection: false,
            allow_sha1: false,
        }
    }
}

impl Configuration {
    /// Indica si ya se ha visto la versión actual del asistente del primer arranque.
    pub fn setup_wizard_seen(&self) -> bool {
        self.setup_wizard_version_seen >= SETUP_WIZARD_VERSION
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SeenVersion {
    Version(u32),
    LegacyFlag(bool),
}

fn seen_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    Ok(match SeenVersion::deserialize(deserializer)? {
        SeenVersion::Version(version) => version,
        SeenVersion::LegacyFlag(seen) => u32::from(seen),
    })
}

#[cfg(test)]
mod tests;
