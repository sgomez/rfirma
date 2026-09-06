//! El comparador de versiones del original, que no es semver.

use std::cmp::Ordering;

/// La versión de AutoFirma que rFirma declara implementar.
pub const IMPLEMENTED_AUTOFIRMA_VERSION: &str = "1.9.2";

/// Una versión legible del protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    parts: Vec<i32>,
    additional_text: String,
}

impl Version {
    /// Parte la cadena, o dice que no tiene forma de versión.
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
            numbers.push(part.parse::<i32>().map_err(|_| MalformedVersion)?);
        }

        let limit = last
            .char_indices()
            .find(|(_, character)| !character.is_ascii_digit())
            .map(|(index, _)| index);

        let (number, additional_text) = match limit {
            Some(index) if index > 0 => (&last[..index], last[index..].to_owned()),
            _ => (*last, String::new()),
        };
        numbers.push(number.parse::<i32>().map_err(|_| MalformedVersion)?);

        Ok(Self {
            parts: numbers,
            additional_text,
        })
    }

    /// Si esta versión es estrictamente mayor que la otra.
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

/// Una cadena que no tiene forma de versión.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MalformedVersion;

fn starts_with_space(text: &str) -> bool {
    text.chars().next().is_some_and(|character| {
        matches!(
            character,
            ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{2028}' | '\u{2029}'
        )
    })
}

#[cfg(test)]
mod tests;
