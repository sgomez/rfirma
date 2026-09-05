//! La comparación de versiones del original, que **no es semver** (ID-251).
//!
//! Es la clase `Version` de `afirma-simple`
//! (`es/gob/afirma/standalone/protocol/Version.java`) reproducida regla a
//! regla. Sus cuatro trampas contradicen semver justo donde importa, y las
//! cuatro tienen prueba aquí aunque ninguna dispare con lo que mandan las sedes
//! —que piden `1.6`, `1.7` y `1.8`— porque son **la razón de que este fichero
//! exista** en vez de un `use semver::Version` (TD-54):
//!
//! | caso | aquí | semver |
//! |---|---|---|
//! | `1.7.0.0` vs `1.7` | mayor: más partes gana | iguales |
//! | `1.7a` vs `1.7` | mayor: el sufijo **suma** | menor: es un *prerelease* |
//! | `1.7 RC1` vs `1.7` | menor: el sufijo empieza por espacio | ni siquiera parsea |
//! | `1.7A` vs `1.7a` | iguales: el sufijo no distingue mayúsculas | distintas |
//!
//! Lo que se compara contra `mcv` es
//! [`IMPLEMENTED_AUTOFIRMA_VERSION`], **no** la versión de rFirma (ID-250).

use std::cmp::Ordering;

/// La versión de AutoFirma que rFirma declara implementar.
///
/// Es un número distinto de la versión de rFirma, y separarlos no es cosmético:
/// con la versión del producto —`0.5`— cualquier sede que exija `mcv=1.9.x`
/// obtendría `0.5 < 1.9.x` y **nunca** se firmaría ahí (ID-250).
///
/// Declararla **no promete un catálogo de operaciones**: una sede que pida
/// `save` se lleva su `SAF_04` igual.
pub const IMPLEMENTED_AUTOFIRMA_VERSION: &str = "1.9.2";

/// Una versión legible partida como la parte el original: números separados por
/// puntos, y un texto adicional pegado al último número.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    parts: Vec<i64>,
    additional_text: String,
}

impl Version {
    /// Parte la cadena, o dice que no tiene forma de versión.
    ///
    /// Reproduce el constructor de `Version.java:21`-`56`, incluidas dos
    /// rarezas de Java que cambian el veredicto:
    ///
    /// - `split("\\.")` **descarta los trozos vacíos del final**, así que
    ///   `1.7.` es `1.7` y no un error;
    /// - todas las partes menos la última tienen que ser un entero entero; la
    ///   última puede ser un entero seguido de texto (`1.7a`), pero si empieza
    ///   por algo que no es un dígito, no parsea.
    pub fn parse(legible: &str) -> Result<Self, MalformedVersion> {
        let mut parts: Vec<&str> = legible.split('.').collect();
        while parts.last().is_some_and(|part| part.is_empty()) {
            parts.pop();
        }
        let Some((last, leading)) = parts.split_last() else {
            return Err(MalformedVersion);
        };

        let mut numbers = Vec::with_capacity(parts.len());
        for part in leading {
            numbers.push(part.parse::<i64>().map_err(|_| MalformedVersion)?);
        }

        let limit = last
            .char_indices()
            .find(|(_, character)| !character.is_ascii_digit())
            .map(|(index, _)| index);

        let (number, additional_text) = match limit {
            Some(index) if index > 0 => (&last[..index], last[index..].to_owned()),
            _ => (*last, String::new()),
        };
        numbers.push(number.parse::<i64>().map_err(|_| MalformedVersion)?);

        Ok(Self {
            parts: numbers,
            additional_text,
        })
    }

    /// `true` si esta versión es **estrictamente mayor** que la otra.
    ///
    /// Es `greaterThan` de `Version.java:120`-`166`, en el mismo orden de
    /// ramas: primero los números parte a parte, luego el número de partes, y
    /// sólo entonces el texto adicional.
    pub fn greater_than(&self, other: &Self) -> bool {
        for (mine, theirs) in self.parts.iter().zip(other.parts.iter()) {
            match mine.cmp(theirs) {
                Ordering::Greater => return true,
                Ordering::Less => return false,
                Ordering::Equal => {}
            }
        }

        match self.parts.len().cmp(&other.parts.len()) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => {}
        }

        self.additional_text_greater_than(&other.additional_text)
    }

    /// Las cinco ramas del texto adicional, tal cual las escribe el original.
    fn additional_text_greater_than(&self, theirs: &str) -> bool {
        let mine = self.additional_text.as_str();

        if mine.eq_ignore_ascii_case(theirs) {
            return false;
        }
        if !starts_with_space(mine) && starts_with_space(theirs) {
            return true;
        }
        if !starts_with_space(theirs) && starts_with_space(mine) {
            return false;
        }
        if mine.is_empty() {
            return false;
        }
        if theirs.is_empty() {
            return true;
        }
        mine.to_lowercase() > theirs.to_lowercase()
    }
}

/// Una cadena que no tiene forma de versión. El lanzador la convierte en
/// `SAF_03`, como cualquier otro fallo de parámetros.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MalformedVersion;

/// `Character.isSpaceChar` del primer carácter: sólo separadores de espacio
/// —el espacio normal y sus parientes—, **no** el tabulador ni el salto de
/// línea, que en Java son `isWhitespace` pero no `isSpaceChar`.
fn starts_with_space(text: &str) -> bool {
    text.chars().next().is_some_and(|character| {
        character == ' '
            || (character.is_whitespace()
                && !matches!(
                    character,
                    '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | '\u{1c}'..='\u{1f}'
                ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn greater(left: &str, right: &str) -> bool {
        Version::parse(left)
            .expect("la izquierda deberia parsear")
            .greater_than(&Version::parse(right).expect("la derecha deberia parsear"))
    }

    #[test]
    fn the_numeric_parts_are_compared_left_to_right() {
        assert!(greater("1.7.0", "1.6.5"));
        assert!(!greater("1.6.5", "1.7.0"));
        assert!(!greater("1.7.0", "1.7.0"));
        assert!(greater("2", "1.9.2"));
    }

    #[test]
    fn more_parts_is_newer_which_is_not_what_semver_says() {
        assert!(greater("1.7.0.0", "1.7"));
        assert!(greater("1.7.0.1", "1.7.0"));
        assert!(!greater("1.7", "1.7.0.0"));
    }

    #[test]
    fn a_text_suffix_adds_which_is_not_what_semver_says() {
        assert!(greater("1.7a", "1.7"));
        assert!(!greater("1.7", "1.7a"));
        assert!(greater("1.7b", "1.7a"));
        assert!(greater("1.7a2", "1.7a1"));
    }

    #[test]
    fn a_suffix_that_starts_with_a_space_subtracts() {
        assert!(!greater("1.7 RC1", "1.7"));
        assert!(greater("1.7", "1.7 RC1"));
        assert!(greater("1.7a", "1.7 RC1"));
        assert!(!greater("1.7 RC1", "1.7a"));
    }

    #[test]
    fn the_suffix_is_compared_ignoring_case() {
        assert!(!greater("1.7A", "1.7a"));
        assert!(!greater("1.7a", "1.7A"));
    }

    #[test]
    fn what_the_public_sector_sites_actually_send_is_older_than_what_is_implemented() {
        let implemented =
            Version::parse(IMPLEMENTED_AUTOFIRMA_VERSION).expect("la nuestra deberia parsear");

        for requested in ["1.6", "1.7", "1.8", "1.9", "1.9.2"] {
            let requested = Version::parse(requested).expect("deberia parsear");
            assert!(
                !requested.greater_than(&implemented),
                "{requested:?} no deberia exigir mas de {IMPLEMENTED_AUTOFIRMA_VERSION}"
            );
        }
    }

    #[test]
    fn a_trailing_dot_is_dropped_like_javas_split_does() {
        assert_eq!(
            Version::parse("1.7.").expect("java la parte igual"),
            Version::parse("1.7").expect("parsea")
        );
    }

    #[test]
    fn what_has_no_shape_of_a_version_does_not_parse() {
        for malformed in ["", "beta", "1.a", "1..7", "uno.siete", "1.7.0.0.beta"] {
            assert_eq!(
                Version::parse(malformed),
                Err(MalformedVersion),
                "{malformed} no es una version"
            );
        }
    }
}
