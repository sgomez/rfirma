//! Los cinco idiomas de la aplicación (ADR-0009, enmendado por el ID-124).
//!
//! Aquí solo vive la *enumeración*: los catálogos de la interfaz son del #55.
//! El texto de la firma visible los necesita antes que la interfaz porque es
//! contenido del PDF, y sigue al idioma de la aplicación en vez de quedarse en
//! castellano fijo como hace AutoFirma (ID-30).

use serde::{Deserialize, Serialize};

/// Idioma de la aplicación: `es`, `ca`, `eu`, `gl` y `en`.
///
/// El valencià salió en v0.3 (ID-124), y no por una decisión sobre lenguas:
/// `Intl.PluralRules("va")` no da la categoría `many` que `es` y `ca` sí usan
/// —cuántas categorías devuelve exactamente depende del CLDR del intérprete—,
/// de modo que ese catálogo está roto para plurales en cuanto los plurales
/// entran. `ca-ES-valencia` sí resuelve a `ca`, pero **no se soportan variantes
/// de ningún idioma**: las reglas de plural se definen sobre el idioma.
///
/// La lista es la misma que la de `src/i18n/locales/`, que sale de `po/`.
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
    /// English.
    #[serde(rename = "en")]
    English,
}

impl Language {
    /// Los cinco, en el orden del ADR-0009 enmendado.
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
            "el valencia salio en v0.3 (ID-124) y no vuelve por la puerta de atras"
        );
    }

    #[test]
    fn gives_every_language_its_own_tag() {
        let tags: HashSet<&str> = Language::ALL.iter().map(|l| l.tag()).collect();
        assert_eq!(tags.len(), Language::ALL.len());
    }

    /// Los cinco `#[serde(rename)]` repiten los cinco brazos de [`Language::tag`],
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
