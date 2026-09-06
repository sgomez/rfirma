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
mod tests;
