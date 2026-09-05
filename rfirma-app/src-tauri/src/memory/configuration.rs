//! La **configuración**: lo que el usuario elige y la aplicación obedece
//! (ID-31, ADR-0010).
//!
//! Cinco de las seis memorias viven aquí —idioma, tema, carpeta de destino y
//! los dos interruptores— y la sexta, la rúbrica, vive **al lado**: es una
//! imagen, no un campo, y se guarda como copia en
//! [`crate::paths::Paths::rubric_path`].
//! Lo importante es lo que **no** hay en esta estructura: ninguna ruta del PNG
//! que eligió el usuario. AutoFirma guarda esa ruta y pierde la rúbrica en
//! silencio en cuanto el fichero se mueve (ID-33).
//!
//! Borrar la configuración no pierde el trabajo, y borrar el estado no
//! reconfigura la aplicación: por eso son dos ficheros y no uno.

use serde::{Deserialize, Serialize};

use crate::destination::DestinationFolder;
use crate::signing::Language;

/// El tema de la ventana: lo que el usuario elige ver.
///
/// [`Theme::System`] no es «claro»: es **no forzar nada** y dejar que mande
/// `prefers-color-scheme`, que es lo que ya hacía la aplicación antes de que
/// este ajuste existiera. Por eso es el valor por omisión y por eso son tres
/// valores y no un interruptor de dos: un booleano no puede decir «lo que diga
/// el sistema».
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

/// Lo que el usuario elige.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Configuration {
    /// El idioma de la interfaz y del texto de la firma visible.
    ///
    /// Por omisión, castellano: en la primera ejecución quien lo decide es el
    /// locale del sistema cotejado contra los publicados (ADR-0010), y eso lo hace
    /// quien arranca la aplicación, no este fichero.
    pub language: Language,
    /// Dónde cae el documento firmado. `None` es «la carpeta de documentos del
    /// usuario», que este módulo no sabe resolver y la interfaz sí.
    pub destination: Option<DestinationFolder>,
    /// «Recordar la última configuración de firma visible». Apagado significa
    /// **no guardarla**, no guardarla y no aplicarla: estado invisible que
    /// reaparece meses después al reencender el interruptor es peor que no
    /// tenerlo.
    pub remember_visible_signature: bool,
    /// «Recordar mi actividad». Cubre los recientes **y** el certificado: son
    /// la misma promesa a quien firma en un ordenador compartido. Al apagarse
    /// **borra** lo ya recordado, y de eso se encarga [`super::Memory`].
    pub remember_activity: bool,
    /// «Avisarme cuando haya una versión nueva» (ID-180). No condiciona la
    /// comprobación —esa sigue corriendo cada 24 h, en Rust, pase lo que
    /// pase—, solo si la ventana enseña la franja con lo que contestó.
    /// Siempre visible en Preferencias, sin la condición que el ID-179
    /// retiró.
    pub notify_new_version: bool,
    /// El tema de la ventana. Ver [`Theme`].
    pub theme: Theme,
    /// Si el aviso del primer arranque (CA local y permiso de red local, #365)
    /// ya se ha descartado. Empieza en `false` y se vuelve `true` para
    /// siempre en cuanto se pulsa «Entendido»: no es un ajuste que se pueda
    /// reactivar desde Preferencias, es una marca de que ya se explicó.
    pub trust_notice_seen: bool,
    /// Si al arrancar se pregunta quién atiende los enlaces `afirma://`
    /// (ID-239). Es el «No volver a preguntar» del banner, guardado del revés
    /// —lo que se guarda es si se sigue preguntando— para que el valor por
    /// omisión sea el que trae la aplicación recién instalada y no haga falta
    /// escribirlo nunca. Se puede volver a encender desde Preferencias: el
    /// banner es la única cosa que se descarta para siempre y **se deshace ahí
    /// mismo**.
    pub ask_about_url_handler: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            language: Language::Spanish,
            destination: None,
            // Los dos activos por omisión, como en la ficha de Preferencias:
            // son mejoras del recorrido, y arrancar con ellas apagadas
            // escondería justo lo que justificó el prototipo.
            remember_visible_signature: true,
            remember_activity: true,
            // Se avisa siempre por omisión: apagarlo es el gesto explícito,
            // no el punto de partida (ID-180).
            notify_new_version: true,
            // Sin elegir, manda el sistema: es lo que hacía la ventana antes
            // de que el ajuste existiera, y una aplicación que se abre en
            // claro dentro de un escritorio oscuro parece rota.
            theme: Theme::System,
            // Sin descartar por omisión: es la primera vez, así que el aviso
            // tiene que aparecer.
            trust_notice_seen: false,
            // Se pregunta: el banner es preventivo por narices (ID-239).
            // Cuando el trámite lo atiende la otra aplicación rFirma ni se
            // ejecuta, así que si no lo pregunta al arrancar no lo pregunta
            // nunca.
            ask_about_url_handler: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: solo serde.
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

    /// Recién instalada, la aplicación pregunta quién atiende `afirma://`:
    /// el silencio se elige, no se hereda (ID-239).
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
