//! Los cinco idiomas soportados por la aplicación (ADR-0009).

use serde::{Deserialize, Serialize};

/// Idioma de la aplicación: `es`, `ca`, `eu`, `gl` y `en`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Español.
    #[serde(rename = "es")]
    Spanish,
    /// Català.
    #[serde(rename = "ca")]
    Catalan,
    /// Euskara.
    #[serde(rename = "eu")]
    Basque,
    /// Galego.
    #[serde(rename = "gl")]
    Galician,
    /// English.
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// Los cinco idiomas soportados.
    pub const ALL: [Self; 5] = [
        Self::Spanish,
        Self::Catalan,
        Self::Basque,
        Self::Galician,
        Self::English,
    ];

    /// Etiqueta corta del idioma.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Spanish => "es",
            Self::Catalan => "ca",
            Self::Basque => "eu",
            Self::Galician => "gl",
            Self::English => "en",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Language;
    use std::collections::HashSet;

    #[test]
    fn covers_the_five_languages_of_the_adr() {
        assert_eq!(Language::ALL.len(), 5);
        assert_eq!(
            Language::ALL.map(Language::tag),
            ["es", "ca", "eu", "gl", "en"],
            "los cinco idiomas no coinciden con los esperados"
        );
    }

    #[test]
    fn gives_every_language_its_own_tag() {
        let tags: HashSet<&str> = Language::ALL.iter().map(|l| l.tag()).collect();
        assert_eq!(tags.len(), Language::ALL.len());
    }

    #[test]
    fn is_persisted_by_the_very_tag_it_reports() {
        for language in Language::ALL {
            assert_eq!(
                serde_json::to_value(language).expect("deberia serializarse"),
                serde_json::json!(language.tag()),
                "el rename de serde y tag() se han separado en {language:?}"
            );
            assert_eq!(
                serde_json::from_value::<Language>(serde_json::json!(language.tag()))
                    .expect("deberia leerse"),
                language,
            );
        }
    }
}
