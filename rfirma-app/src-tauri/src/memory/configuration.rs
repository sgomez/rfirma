//! Estructura de configuración persistida entre sesiones (ADR-0010).

use serde::{Deserialize, Serialize};

use crate::destination::DestinationFolder;
use crate::signing::Language;

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
    /// Indica si el aviso inicial sobre la CA local ya se descartó.
    pub trust_notice_seen: bool,
    /// Indica si se debe consultar el manejador del protocolo al arrancar.
    pub ask_about_url_handler: bool,
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
            trust_notice_seen: false,
            ask_about_url_handler: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_switches_start_on() {
        let configuration = Configuration::default();

        assert!(configuration.remember_visible_signature);
        assert!(configuration.remember_activity);
    }

    #[test]
    fn notify_new_version_starts_on() {
        assert!(Configuration::default().notify_new_version);
    }

    #[test]
    fn the_url_handler_is_asked_about_by_default() {
        assert!(Configuration::default().ask_about_url_handler);
    }

    #[test]
    fn the_trust_notice_has_not_been_seen_by_default() {
        assert!(!Configuration::default().trust_notice_seen);
    }

    #[test]
    fn without_choosing_the_theme_is_the_one_the_system_says() {
        assert_eq!(Configuration::default().theme, Theme::System);
    }

    #[test]
    fn the_theme_is_persisted_in_lowercase() {
        let written = serde_json::to_value(Configuration {
            theme: Theme::Dark,
            ..Configuration::default()
        })
        .expect("deberia serializarse");

        assert_eq!(written["theme"], serde_json::json!("dark"));
    }

    #[test]
    fn the_language_is_persisted_as_its_short_tag() {
        let written = serde_json::to_value(Configuration {
            language: Language::Basque,
            ..Configuration::default()
        })
        .expect("deberia serializarse");

        assert_eq!(written["language"], serde_json::json!("eu"));
    }

    #[test]
    fn a_configuration_missing_a_field_takes_the_default_for_it() {
        let configuration: Configuration =
            serde_json::from_str(r#"{"language": "gl"}"#).expect("deberia leerse");

        assert_eq!(configuration.language, Language::Galician);
        assert!(configuration.remember_activity);
        assert!(configuration.notify_new_version);
    }

    #[test]
    fn the_configuration_holds_no_path_to_the_rubric_the_user_chose() {
        let written = serde_json::to_value(Configuration::default()).expect("deberia serializarse");

        let fields: Vec<&str> = written
            .as_object()
            .expect("deberia ser un objeto")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            vec![
                "ask_about_url_handler",
                "destination",
                "language",
                "notify_new_version",
                "remember_activity",
                "remember_visible_signature",
                "theme",
                "trust_notice_seen",
            ],
            "la rubrica es una copia en el almacen, nunca un campo con la ruta del original"
        );
    }
}
