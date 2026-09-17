//! El catálogo declarativo de la suite de conformidad, leído de `catalogue.toml`: los metadatos de
//! cada exigencia, no su cuerpo ejecutable.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::baseline::Expectation;

/// El vocabulario cerrado de `suite`; los conjuntos que falten los abren sus propios tickets.
pub(crate) const THE_SUITES: &[&str] = &[
    "saludo",
    "transporte.websocket",
    "transporte.service",
    "versiones",
    "operaciones",
    "operaciones.firma",
    "errores",
];

/// Lo que necesita una comprobación además del sujeto, tal y como se declara en `needs`.
pub(crate) const A_PERSON: &str = "persona";
pub(crate) const A_STORE: &str = "almacén:";
pub(crate) const SOME_PORTS: &str = "puertos:";
pub(crate) const A_WAIT: &str = "espera:";

/// Cómo se conduce al cliente publicado para ejercitar la exigencia.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Drive {
    pub mode: String,
    pub script: String,
}

/// Una exigencia del protocolo con todo lo que se sabe de ella menos cómo se mide.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Check {
    pub id: String,
    pub suite: String,
    pub chapter: String,
    pub citation: String,
    pub statement: String,
    #[serde(default)]
    pub drive: Option<Drive>,
    #[serde(default)]
    pub harness: Option<String>,
    #[serde(default)]
    pub expects_saf: Option<String>,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub unmeasurable: Option<String>,
    /// La línea base: qué se espera de cada perfil de sujeto, por el nombre del perfil.
    #[serde(default)]
    pub expect: BTreeMap<String, Expectation>,
}

#[derive(Debug, Deserialize)]
struct Catalogue {
    check: Vec<Check>,
}

impl Check {
    pub(crate) fn needs_a_person(&self) -> bool {
        self.needs.iter().any(|need| need == A_PERSON)
    }

    pub(crate) fn required_store(&self) -> Option<&str> {
        self.needs
            .iter()
            .find_map(|need| need.strip_prefix(A_STORE))
    }

    pub(crate) fn required_ports(&self) -> Vec<u16> {
        self.needs
            .iter()
            .find_map(|need| need.strip_prefix(SOME_PORTS))
            .map(|list| {
                list.split(',')
                    .filter_map(|port| port.trim().parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn declared_patience(&self) -> Option<Duration> {
        self.needs
            .iter()
            .find_map(|need| need.strip_prefix(A_WAIT))
            .and_then(|seconds| seconds.trim().parse().ok())
            .map(Duration::from_secs)
    }
}

/// Dónde vive el catálogo: un fichero del árbol, no un literal empotrado, porque es lo que se lee
/// y se revisa.
pub(crate) fn the_catalogue_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/conformance/catalogue.toml")
}

pub(crate) fn read_the_catalogue() -> Result<Vec<Check>, String> {
    let path = the_catalogue_file();
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    the_catalogue_in(&raw).map_err(|complaint| format!("{}: {complaint}", path.display()))
}

pub(crate) fn the_catalogue_in(raw: &str) -> Result<Vec<Check>, String> {
    let catalogue: Catalogue =
        toml::from_str(raw).map_err(|error| format!("no es un catálogo válido: {error}"))?;
    Ok(catalogue
        .check
        .into_iter()
        .map(|check| Check {
            statement: as_one_line(&check.statement),
            ..check
        })
        .collect())
}

/// El enunciado, dicho de corrido: en el catálogo va partido en líneas para que se lea, y en la
/// tarjeta va en una sola.
fn as_one_line(statement: &str) -> String {
    statement.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const AN_ENTRY: &str = r#"
[[check]]
id = "an_origin_that_is_not_local_is_rejected"
suite = "transporte.websocket"
chapter = "05"
citation = "AfirmaWebSocketServerV4.java:57-68"
statement = """
El canal responde SAF_47 a cualquier origen que no sea 127.0.0.1.
"""
drive = { mode = "v4-ipv6", script = "selectcert" }
expects_saf = "SAF_47"
needs = ["persona", "almacén:rfirma-test-ecc", "puertos:63131, 63132", "espera:90"]
question = "¿se pidió el PIN? [s/n]"
"#;

    #[test]
    fn reads_an_entry_with_every_field() {
        let checks = the_catalogue_in(AN_ENTRY).unwrap();
        let check = &checks[0];
        assert_eq!(check.id, "an_origin_that_is_not_local_is_rejected");
        assert_eq!(check.suite, "transporte.websocket");
        assert_eq!(check.chapter, "05");
        assert_eq!(check.expects_saf.as_deref(), Some("SAF_47"));
        assert_eq!(
            check.drive.as_ref().unwrap(),
            &Drive {
                mode: "v4-ipv6".to_owned(),
                script: "selectcert".to_owned()
            }
        );
    }

    #[test]
    fn the_statement_arrives_without_the_newlines_of_its_block() {
        let checks = the_catalogue_in(AN_ENTRY).unwrap();
        assert_eq!(
            checks[0].statement,
            "El canal responde SAF_47 a cualquier origen que no sea 127.0.0.1."
        );
    }

    #[test]
    fn reads_what_a_check_needs_from_its_declaration() {
        let checks = the_catalogue_in(AN_ENTRY).unwrap();
        let check = &checks[0];
        assert!(check.needs_a_person());
        assert_eq!(check.required_store(), Some("rfirma-test-ecc"));
        assert_eq!(check.required_ports(), vec![63131, 63132]);
        assert_eq!(check.declared_patience(), Some(Duration::from_secs(90)));
    }

    #[test]
    fn a_check_that_needs_nothing_says_so() {
        let checks = the_catalogue_in(
            r#"
[[check]]
id = "an_id"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Algo se rechaza con SAF_03."
"#,
        )
        .unwrap();
        let check = &checks[0];
        assert!(!check.needs_a_person());
        assert_eq!(check.required_store(), None);
        assert!(check.required_ports().is_empty());
        assert_eq!(check.declared_patience(), None);
    }

    #[test]
    fn a_malformed_catalogue_complains_instead_of_parsing_half() {
        assert!(the_catalogue_in("[[check]]\nid = ").is_err());
    }

    #[test]
    fn the_catalogue_of_the_repository_reads() {
        let checks = read_the_catalogue().unwrap();
        assert_eq!(checks.len(), 65);
    }
}
