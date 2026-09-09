//! Una URL `afirma://` partida en un verbo y unos pares.

use std::collections::BTreeMap;

use super::refusal::Refusal;

const SCHEME: &str = "afirma://";

const LONGEST_TRACED_VALUE: usize = 80;

/// Una URL `afirma://` ya partida.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfirmaUrl {
    verb: String,
    parameters: BTreeMap<String, String>,
}

impl AfirmaUrl {
    /// Si la cadena viene por el esquema del protocolo, esté bien formada o no.
    pub fn is_a_protocol_url(candidate: &str) -> bool {
        strip_scheme(candidate).is_some()
    }

    /// Parte la cadena, o dice por qué no es una URL del protocolo.
    pub fn parse(url: &str) -> Result<Self, Refusal> {
        let Some(rest) = strip_scheme(url) else {
            return Err(Refusal::params(format!(
                "la invocacion no empieza por {SCHEME}: {url}"
            )));
        };

        let (verb, query) = match rest.split_once('?') {
            Some((verb, query)) => (verb, query),
            None => (rest, ""),
        };

        if verb.is_empty() {
            return Err(Refusal::params(format!(
                "la invocacion no trae verbo: {url}"
            )));
        }

        let mut parameters = BTreeMap::new();
        for pair in query.split('&') {
            let Some(position) = pair.find('=') else {
                continue;
            };
            if position == 0 {
                continue;
            }
            let (key, value) = pair.split_at(position);
            parameters.insert(key.to_owned(), url_decode(&value[1..]));
        }

        Ok(Self {
            verb: verb.to_owned(),
            parameters,
        })
    }

    /// Lo que va entre el esquema y el `?`: `websocket`, `sign`, `selectcert`…
    pub fn verb(&self) -> &str {
        &self.verb
    }

    /// Una operación armada desde pares ya descodificados, la forma en la que llega el XML de
    /// parámetros del servidor intermedio.
    pub fn of(verb: &str, pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            verb: verb.to_owned(),
            parameters: pairs.into_iter().collect(),
        }
    }

    /// El valor de un parámetro, ya descodificado, si vino.
    pub fn parameter(&self, name: &str) -> Option<&str> {
        self.parameters.get(name).map(String::as_str)
    }

    /// La misma URL en una línea, con los valores largos reducidos a su tamaño.
    pub fn abridged(&self) -> String {
        let mut line = format!("{SCHEME}{}", self.verb);
        let mut separator = '?';
        for (name, value) in &self.parameters {
            line.push(separator);
            separator = '&';
            line.push_str(name);
            line.push('=');
            line.push_str(&abridged_value(value));
        }
        line
    }

    /// La misma URL con un parámetro añadido o sustituido (servidor intermedio: `dat` resuelto).
    pub fn with_parameter(mut self, name: &str, value: String) -> Self {
        self.parameters.insert(name.to_owned(), value);
        self
    }
}

pub(super) fn abridged_value(value: &str) -> String {
    let size = value.chars().count();
    if size <= LONGEST_TRACED_VALUE {
        return value.to_owned();
    }
    format!("<{size} caracteres>")
}

/// Quita `afirma://` sin distinguir mayúsculas, o dice que no estaba.
fn strip_scheme(url: &str) -> Option<&str> {
    let head = url.get(..SCHEME.len())?;
    head.eq_ignore_ascii_case(SCHEME)
        .then(|| &url[SCHEME.len()..])
}

pub(super) fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => match hexadecimal_byte(bytes.get(index + 1..index + 3)) {
                Some(byte) => {
                    decoded.push(byte);
                    index += 3;
                }
                None => {
                    decoded.push(b'%');
                    index += 1;
                }
            },
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

/// Los dos dígitos hexadecimales de un `%XX`, si los dos lo son.
fn hexadecimal_byte(digits: Option<&[u8]>) -> Option<u8> {
    let digits = digits?;
    if digits.len() != 2 {
        return None;
    }
    let high = (digits[0] as char).to_digit(16)?;
    let low = (digits[1] as char).to_digit(16)?;
    Some((high * 16 + low) as u8)
}

#[cfg(test)]
mod tests;
