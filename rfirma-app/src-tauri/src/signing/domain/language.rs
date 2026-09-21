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

    /// El idioma de un locale POSIX (`ca_ES.UTF-8`) o BCP 47 (`en-GB`), si es uno de los cinco.
    pub fn of_locale(locale: &str) -> Option<Self> {
        let primary = locale.split(['_', '-', '.', '@']).next()?;
        Self::ALL
            .into_iter()
            .find(|language| language.tag().eq_ignore_ascii_case(primary))
    }

    /// El primero de los locales preferidos que sea uno de los cinco, o castellano.
    pub fn first_of<S: AsRef<str>>(locales: impl IntoIterator<Item = S>) -> Self {
        locales
            .into_iter()
            .find_map(|locale| Self::of_locale(locale.as_ref()))
            .unwrap_or(Self::Spanish)
    }
}

#[cfg(test)]
mod tests;
