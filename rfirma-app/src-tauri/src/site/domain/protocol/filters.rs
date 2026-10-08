//! La expresión de filtro que manda la sede, que cruza entera al motor, la biblioteca PKCS#11 a la que acota el listado y el catálogo de criterios medidos contra el original (ADR-0022).

use super::key_store::StoreScope;

const FILTER: &str = "filter";
const FILTERS: &str = "filters";

/// Los criterios medidos contra el original.
pub const ACCEPTED_CRITERIA: &[&str] = &[
    "authcert:",
    "dnie:",
    "encodedcert:",
    "issuer.contains:",
    "issuer.rfc2254.recurse:",
    "issuer.rfc2254:",
    "keyusage.",
    "nonexpired:",
    "policyid:",
    "pseudonym:",
    "qualified:",
    "signingcert:",
    "ssl:",
    "sscd:",
    "subject.contains:",
    "subject.rfc2254:",
    "thumbprint:",
];

/// Criterio sin argumento satisfecho por construcción.
pub const SATISFIED_BY_CONSTRUCTION: &str = "disableopeningexternalstores";

/// Lo que la sede pide del listado, listo para cruzar al motor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteFilter {
    declared: Vec<(String, String)>,
    scope: StoreScope,
}

impl SiteFilter {
    /// El mismo filtro, acotado a los almacenes que nombra la sede.
    pub fn within(self, scope: StoreScope) -> Self {
        Self { scope, ..self }
    }

    /// Los almacenes a los que la sede acota el listado.
    pub fn scope(&self) -> &StoreScope {
        &self.scope
    }

    /// Si la sede no declaró ningún filtro.
    pub fn declares_nothing(&self) -> bool {
        self.declared.is_empty()
    }

    /// Las claves declaradas, en el orden en que se recogieron.
    pub fn declared(&self) -> &[(String, String)] {
        &self.declared
    }

    /// El bloque `java.util.Properties` que recibe el puente.
    pub fn as_java_properties(&self) -> String {
        let mut block = String::new();
        for (key, value) in &self.declared {
            block.push_str(key);
            block.push('=');
            for character in value.chars() {
                match character {
                    '\\' => block.push_str("\\\\"),
                    '\n' => block.push_str("\\n"),
                    '\r' => block.push_str("\\r"),
                    other => block.push(other),
                }
            }
            block.push('\n');
        }
        block
    }
}

/// Lo que la sede pide del listado, sin las expresiones en las que el original no reconoce nada (ADR-0023).
pub fn site_filter(properties: &[(String, String)]) -> SiteFilter {
    SiteFilter {
        declared: declared_keys(properties)
            .into_iter()
            .filter(|(_, expression)| names_a_criterion(expression))
            .collect(),
        scope: StoreScope::Everywhere,
    }
}

fn names_a_criterion(expression: &str) -> bool {
    expression.split(';').any(|condition| {
        let condition = condition.to_lowercase();
        ACCEPTED_CRITERIA
            .iter()
            .any(|criterion| condition.starts_with(criterion))
    })
}

/// Las claves de filtro que la sede declaró, con la precedencia del original.
fn declared_keys(properties: &[(String, String)]) -> Vec<(String, String)> {
    if let Some(value) = value_of(properties, FILTER) {
        return vec![(FILTER.to_owned(), value.to_owned())];
    }
    if let Some(value) = value_of(properties, FILTERS) {
        return vec![(FILTERS.to_owned(), value.to_owned())];
    }

    let mut numbered = Vec::new();
    for index in 1.. {
        let key = format!("{FILTERS}.{index}");
        let Some(value) = value_of(properties, &key) else {
            break;
        };
        numbered.push((key, value.to_owned()));
    }
    numbered
}

fn value_of<'a>(properties: &'a [(String, String)], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

#[cfg(test)]
mod tests;
