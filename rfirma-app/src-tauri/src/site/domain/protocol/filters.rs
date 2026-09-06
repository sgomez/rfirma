//! La expresión de filtro que manda la sede: qué se deja pasar al motor.

use super::refusal::{Refusal, RefusalSituation};

const FILTER: &str = "filter";
const FILTERS: &str = "filters";

/// Los criterios que rFirma deja cruzar al motor.
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

/// Criterio sin argumento aceptado por compatibilidad.
pub const SATISFIED_BY_CONSTRUCTION: &str = "disableopeningexternalstores";

/// Criterios aceptados sin cobertura de su veredicto.
pub const UNMEASURED_CRITERIA: &[&str] = &["dnie:", "pseudonym:", "qualified:", "ssl:"];

/// Lo que la sede pide del listado, listo para cruzar al motor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SiteFilter {
    declared: Vec<(String, String)>,
}

impl SiteFilter {
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

/// Lo que la sede pide del listado, o por qué no se le sirve.
pub fn site_filter(properties: &[(String, String)]) -> Result<SiteFilter, Refusal> {
    let declared = declared_keys(properties);

    for (key, expression) in &declared {
        for criterion in expression.split(';') {
            check_is_accepted(key, criterion)?;
        }
    }

    Ok(SiteFilter { declared })
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

fn check_is_accepted(key: &str, criterion: &str) -> Result<(), Refusal> {
    let trimmed = criterion.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let lowercase = trimmed.to_ascii_lowercase();

    if lowercase == SATISFIED_BY_CONSTRUCTION {
        return Ok(());
    }
    if ACCEPTED_CRITERIA
        .iter()
        .any(|accepted| lowercase.starts_with(accepted))
    {
        return Ok(());
    }

    Err(Refusal::params(format!(
        "el criterio de filtro '{trimmed}' de '{key}' no esta en la lista blanca"
    ))
    .because(RefusalSituation::UnsupportedFilter))
}

#[cfg(test)]
mod tests;
