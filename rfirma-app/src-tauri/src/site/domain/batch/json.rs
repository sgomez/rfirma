//! Un JSON de lectura y escritura con el orden del documento, y no el alfabético de `serde_json::Value`.

use std::fmt;

/// Un valor JSON, con sus objetos en el orden en que se leyeron.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    Null,
    Bool(bool),
    /// El literal numérico tal cual, para no perder precisión al reescribirlo.
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

/// Un JSON mal formado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError(String);

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsonError {}

impl Json {
    /// Lee un documento JSON completo.
    pub fn parse(text: &str) -> Result<Self, JsonError> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            position: 0,
        };
        parser.skip_whitespace();
        let value = parser.parse_value()?;
        parser.skip_whitespace();
        if parser.position != parser.bytes.len() {
            return Err(JsonError("sobra texto tras el JSON".to_owned()));
        }
        Ok(value)
    }

    /// El objeto, si lo es.
    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// El objeto mutable, si lo es.
    pub fn as_object_mut(&mut self) -> Option<&mut Vec<(String, Json)>> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// La lista, si lo es.
    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// La lista mutable, si lo es.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Json>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// La cadena, si lo es.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// El booleano, si lo es.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// El valor de una clave, si el valor es un objeto y la tiene.
    pub fn get(&self, key: &str) -> Option<&Json> {
        self.as_object()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Quita una clave de un objeto, y da su valor si la tenía.
    pub fn remove(&mut self, key: &str) -> Option<Json> {
        let Self::Object(entries) = self else {
            return None;
        };
        let index = entries.iter().position(|(k, _)| k == key)?;
        Some(entries.remove(index).1)
    }

    /// Añade una clave a un objeto, o sustituye el valor si ya existía.
    pub fn set(&mut self, key: &str, value: Json) {
        let Self::Object(entries) = self else {
            return;
        };
        match entries.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_owned(), value)),
        }
    }

    /// El JSON serializado, sin espacios, con las claves en el orden en que se guardaron.
    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(true) => out.push_str("true"),
            Self::Bool(false) => out.push_str("false"),
            Self::Number(literal) => out.push_str(literal),
            Self::String(value) => write_json_string(value, out),
            Self::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Self::Object(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_json_string(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_json_string(value: &str, out: &mut String) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.position),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.position += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), JsonError> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(JsonError(format!(
                "se esperaba '{}' en la posicion {}",
                byte as char, self.position
            )))
        }
    }

    fn parse_value(&mut self) -> Result<Json, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::String(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", Json::Bool(true)),
            Some(b'f') => self.parse_literal("false", Json::Bool(false)),
            Some(b'n') => self.parse_literal("null", Json::Null),
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.parse_number(),
            _ => Err(JsonError(format!(
                "se esperaba un valor JSON en la posicion {}",
                self.position
            ))),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.position..].starts_with(literal.as_bytes()) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(JsonError(format!(
                "se esperaba '{literal}' en la posicion {}",
                self.position
            )))
        }
    }

    fn parse_number(&mut self) -> Result<Json, JsonError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-'))
        {
            self.position += 1;
        }
        let literal = std::str::from_utf8(&self.bytes[start..self.position])
            .map_err(|error| JsonError(error.to_string()))?
            .to_owned();
        Ok(Json::Number(literal))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut value = String::new();
        loop {
            match self.peek() {
                None => return Err(JsonError("cadena JSON sin cerrar".to_owned())),
                Some(b'"') => {
                    self.position += 1;
                    return Ok(value);
                }
                Some(b'\\') => {
                    self.position += 1;
                    match self.peek() {
                        Some(b'"') => value.push('"'),
                        Some(b'\\') => value.push('\\'),
                        Some(b'/') => value.push('/'),
                        Some(b'n') => value.push('\n'),
                        Some(b'r') => value.push('\r'),
                        Some(b't') => value.push('\t'),
                        Some(b'b') => value.push('\u{8}'),
                        Some(b'f') => value.push('\u{c}'),
                        Some(b'u') => {
                            let hex = self
                                .bytes
                                .get(self.position + 1..self.position + 5)
                                .ok_or_else(|| JsonError("escape '\\u' incompleto".to_owned()))?;
                            let hex = std::str::from_utf8(hex)
                                .map_err(|error| JsonError(error.to_string()))?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|error| JsonError(error.to_string()))?;
                            value.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                            self.position += 4;
                        }
                        _ => return Err(JsonError("escape JSON desconocido".to_owned())),
                    }
                    self.position += 1;
                }
                Some(_) => {
                    let rest = &self.bytes[self.position..];
                    let width = utf8_char_width(rest[0]);
                    let slice = rest
                        .get(..width)
                        .ok_or_else(|| JsonError("cadena JSON truncada".to_owned()))?;
                    value.push_str(
                        std::str::from_utf8(slice).map_err(|error| JsonError(error.to_string()))?,
                    );
                    self.position += width;
                }
            }
        }
    }

    fn parse_array(&mut self) -> Result<Json, JsonError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b']') => {
                    self.position += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(JsonError("lista JSON mal formada".to_owned())),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Json, JsonError> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(JsonError("objeto JSON mal formado".to_owned())),
            }
        }
    }
}

/// El número de bytes UTF-8 de un carácter a partir de su primer byte.
fn utf8_char_width(first_byte: u8) -> usize {
    if first_byte & 0x80 == 0 {
        1
    } else if first_byte & 0xE0 == 0xC0 {
        2
    } else if first_byte & 0xF0 == 0xE0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests;
