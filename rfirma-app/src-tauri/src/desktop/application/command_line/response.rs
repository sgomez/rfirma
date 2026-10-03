//! La respuesta de una orden como estructura, de la que sale el XML de `--xml`; no decide qué lleva.

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

impl Response {
    pub(super) fn new(result: &'static str, fields: Vec<Field>) -> Self {
        Self { result, fields }
    }

    pub(super) fn to_xml(&self) -> Vec<u8> {
        let fields: String = self.fields.iter().map(Field::to_xml).collect();
        format!(
            "<afirma><result>{}</result><response>{fields}</response></afirma>\n",
            self.result
        )
        .into_bytes()
    }
}

impl Field {
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
