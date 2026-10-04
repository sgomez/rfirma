//! La respuesta de una orden como estructura, de la que salen el XML de `--xml` y el JSON de `--json` de las órdenes que aún no tienen el suyo; no decide qué lleva.

use serde_json::{Map, Value};

/// El resultado y los campos de una respuesta, en el orden en que se serializan.
pub(super) struct Response {
    result: &'static str,
    fields: Vec<Field>,
}

/// Un campo de la respuesta: uno solo, o uno que puede repetirse y es una lista.
pub(super) enum Field {
    One(&'static str, String),
    Many(&'static str, Vec<String>),
}

/// El documento en que se serializa una respuesta.
#[derive(Clone, Copy)]
pub(super) enum Document {
    Xml,
    Json,
}

impl Response {
    pub(super) fn new(result: &'static str, fields: Vec<Field>) -> Self {
        Self { result, fields }
    }

    pub(super) fn render(&self, document: Document) -> Vec<u8> {
        match document {
            Document::Xml => self.to_xml(),
            Document::Json => self.to_json(),
        }
    }

    fn to_json(&self) -> Vec<u8> {
        let mut response = Map::new();
        for field in &self.fields {
            field.add_to(&mut response);
        }
        let mut afirma = Map::new();
        afirma.insert("result".to_owned(), Value::String(self.result.to_owned()));
        afirma.insert("response".to_owned(), Value::Object(response));
        let mut root = Map::new();
        root.insert("afirma".to_owned(), Value::Object(afirma));
        let mut bytes = Value::Object(root).to_string().into_bytes();
        bytes.push(b'\n');
        bytes
    }

    fn to_xml(&self) -> Vec<u8> {
        let fields: String = self.fields.iter().map(Field::to_xml).collect();
        format!(
            "<afirma><result>{}</result><response>{fields}</response></afirma>\n",
            self.result
        )
        .into_bytes()
    }
}

impl Field {
    fn add_to(&self, response: &mut Map<String, Value>) {
        let (name, value) = match self {
            Self::One(name, value) => (name, Value::String(value.clone())),
            Self::Many(name, values) => (
                name,
                Value::Array(values.iter().cloned().map(Value::String).collect()),
            ),
        };
        response.insert((*name).to_owned(), value);
    }

    fn to_xml(&self) -> String {
        match self {
            Self::One(name, value) => element(name, value),
            Self::Many(name, values) => values.iter().map(|value| element(name, value)).collect(),
        }
    }
}

fn element(name: &str, value: &str) -> String {
    format!("<{name}>{}</{name}>", escaped_for_xml(value))
}

fn escaped_for_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
