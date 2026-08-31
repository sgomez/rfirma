//! Los seis idiomas de la aplicación (ADR-0009).
//!
//! Aquí solo vive la *enumeración*: los catálogos de la interfaz son del #55.
//! El texto de la firma visible los necesita antes que la interfaz porque es
//! contenido del PDF, y sigue al idioma de la aplicación en vez de quedarse en
//! castellano fijo como hace AutoFirma (ID-30).

use serde::{Deserialize, Serialize};

/// Idioma de la aplicación. La lista se toma entera, no por partes: un
/// subconjunto de las lenguas cooficiales no es una decisión técnica.
///
/// Se persiste por su [`Language::tag`] —`"es"`, `"ca"`…— y no por el nombre de
/// la variante: el fichero de configuración lo escribe rFirma pero lo lee
/// cualquiera que abra un informe de fallo, y `"spanish"` no es lo que dice un
/// locale.
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
    /// Valencià. AutoFirma lo mantiene como locale propio (`va_ES`) pese a ser
    /// en la práctica el català con variantes léxicas; aquí igual.
    #[serde(rename = "va")]
    Valencian,
    /// English.
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// Los seis, en el orden del ADR-0009.
    pub const ALL: [Self; 6] = [
        Self::Spanish,
        Self::Catalan,
        Self::Basque,
        Self::Galician,
        Self::Valencian,
        Self::English,
    ];

    /// Etiqueta corta del idioma.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Spanish => "es",
            Self::Catalan => "ca",
            Self::Basque => "eu",
            Self::Galician => "gl",
            Self::Valencian => "va",
            Self::English => "en",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Language;
    use std::collections::HashSet;

    #[test]
    fn covers_the_six_languages_of_the_adr() {
        assert_eq!(Language::ALL.len(), 6);
    }

    #[test]
    fn gives_every_language_its_own_tag() {
        let tags: HashSet<&str> = Language::ALL.iter().map(|l| l.tag()).collect();
        assert_eq!(tags.len(), Language::ALL.len());
    }

    /// Los seis `#[serde(rename)]` repiten los seis brazos de [`Language::tag`],
    /// y dos listas iguales escritas dos veces se separan. Esto las ata: cambiar
    /// una sin la otra pone el PR en rojo.
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
