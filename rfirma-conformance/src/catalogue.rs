//! El catálogo declarativo de la suite de conformidad, leído de `catalogue/`, un fichero por
//! conjunto: los metadatos de cada exigencia, no su cuerpo ejecutable.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use crate::harness::{the_harness_named, Harness};

/// El vocabulario cerrado de `set`, en el orden en que se leen sus ficheros.
pub(crate) const THE_SETS: &[&str] = &[
    "saludo",
    "transporte.websocket",
    "transporte.service",
    "versiones",
    "operaciones",
    "errores",
    "operaciones.firma",
    "operaciones.disco",
    "operaciones.lote",
    "parametros",
];

/// Lo que necesita una comprobación además del cliente, tal y como se declara en `needs`.
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
    pub set: String,
    pub chapter: String,
    pub citation: String,
    pub statement: String,
    #[serde(default)]
    pub drive: Option<Drive>,
    #[serde(default, deserialize_with = "a_registered_harness")]
    pub harness: Option<&'static Harness>,
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
    /// Si es el saludo de su conjunto: lo abre y, si no se cumple, el resto no se corre.
    #[serde(default)]
    pub greeting: bool,
}

fn a_registered_harness<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<&'static Harness>, D::Error> {
    let name = String::deserialize(deserializer)?;
    the_harness_named(&name)
        .map(Some)
        .ok_or_else(|| serde::de::Error::custom(format!("el arnés «{name}» no existe")))
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

/// Dónde vive el catálogo: un directorio con un fichero por conjunto, no un literal empotrado,
/// porque es lo que se lee y se revisa.
pub(crate) fn the_catalogue_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalogue")
}

/// El fichero del conjunto dado, con el mismo nombre que `THE_SETS` salvo que sus puntos se
/// vuelven guiones.
fn the_set_file(set: &str) -> PathBuf {
    the_catalogue_dir().join(format!("{}.toml", set.replace('.', "-")))
}

/// El catálogo entero, o por qué no arranca la suite: cada queja nombra la entrada y lo que le
/// falta.
pub(crate) fn read_the_catalogue() -> Result<Vec<Check>, String> {
    let mut checks = Vec::new();
    for set in THE_SETS {
        let path = the_set_file(set);
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
        let entries = the_catalogue_in(&raw)
            .map_err(|complaint| format!("{}: {complaint}", path.display()))?;
        checks.extend(entries);
    }
    let complaints = complaints_about(&checks);
    if complaints.is_empty() {
        Ok(checks)
    } else {
        Err(format!(
            "el catálogo está mal formado:\n  {}",
            complaints.join("\n  ")
        ))
    }
}

fn complaints_about(checks: &[Check]) -> Vec<String> {
    [
        repeated_ids(checks),
        sets_outside_the_vocabulary(checks),
        empty_fields(checks),
        malformed_unmeasurable_entries(checks),
        entries_without_a_body(checks),
        greetings_that_do_not_open_their_set(checks),
        person_entries_without_a_question_or_a_warning(checks),
    ]
    .concat()
}

fn repeated_ids(checks: &[Check]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    checks
        .iter()
        .filter(|check| !seen.insert(check.id.as_str()))
        .map(|check| format!("{}: id repetido", check.id))
        .collect()
}

fn sets_outside_the_vocabulary(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| !THE_SETS.contains(&check.set.as_str()))
        .map(|check| format!("{}: el conjunto «{}» no existe", check.id, check.set))
        .collect()
}

fn empty_fields(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .flat_map(|check| {
            [
                ("id", &check.id),
                ("set", &check.set),
                ("chapter", &check.chapter),
                ("citation", &check.citation),
                ("statement", &check.statement),
            ]
            .into_iter()
            .filter(|(_, value)| value.trim().is_empty())
            .map(|(field, _)| format!("{}: «{field}» vacío", check.id))
        })
        .collect()
}

fn malformed_unmeasurable_entries(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter_map(|check| match check.unmeasurable.as_deref() {
            Some(motive) if motive.trim().is_empty() => {
                Some(format!("{}: no medible sin motivo", check.id))
            }
            Some(_) if check.drive.is_some() => {
                Some(format!("{}: no medible pero conducida", check.id))
            }
            _ => None,
        })
        .collect()
}

fn entries_without_a_body(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.drive.is_none() && check.unmeasurable.is_none())
        .map(|check| format!("{}: ni se conduce ni se declara no medible", check.id))
        .collect()
}

fn greetings_that_do_not_open_their_set(checks: &[Check]) -> Vec<String> {
    let mut opened = BTreeSet::new();
    checks
        .iter()
        .filter_map(|check| {
            let first_of_its_set = opened.insert(check.set.as_str());
            match check.greeting {
                true if !first_of_its_set => {
                    Some(format!("{}: saludo que no abre su conjunto", check.id))
                }
                true if check.drive.is_none() => Some(format!("{}: saludo sin conducir", check.id)),
                _ => None,
            }
        })
        .collect()
}

fn person_entries_without_a_question_or_a_warning(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.needs_a_person())
        .flat_map(|check| {
            [("pregunta", &check.question), ("aviso", &check.warning)]
                .into_iter()
                .filter(|(_, said)| said.as_deref().is_none_or(|said| said.trim().is_empty()))
                .map(|(what, _)| format!("{}: necesita a una persona y no trae {what}", check.id))
        })
        .collect()
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
set = "transporte.websocket"
chapter = "05"
citation = "AfirmaWebSocketServerV4.java:57-68"
statement = """
El canal responde SAF_47 a cualquier origen que no sea 127.0.0.1.
"""
drive = { mode = "v4-ipv6", script = "selectcert" }
expects_saf = "SAF_47"
needs = ["persona", "almacén:rfirma-test-ecc", "puertos:63131, 63132", "espera:90"]
question = "¿se pidió el PIN? [s/n]"
greeting = true
"#;

    #[test]
    fn reads_an_entry_with_every_field() {
        let checks = the_catalogue_in(AN_ENTRY).unwrap();
        let check = &checks[0];
        assert_eq!(check.id, "an_origin_that_is_not_local_is_rejected");
        assert_eq!(check.set, "transporte.websocket");
        assert_eq!(check.chapter, "05");
        assert_eq!(check.expects_saf.as_deref(), Some("SAF_47"));
        assert!(check.greeting);
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
set = "errores"
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
        assert_eq!(checks.len(), 160);
    }

    #[test]
    fn the_catalogue_orders_its_blocks_like_the_sets_vocabulary() {
        let checks = read_the_catalogue().unwrap();
        let mut blocks: Vec<&str> = Vec::new();
        for check in &checks {
            if blocks.last() != Some(&check.set.as_str()) {
                blocks.push(&check.set);
            }
        }
        assert_eq!(blocks, THE_SETS);
    }

    fn entries(raw: &str) -> Vec<Check> {
        the_catalogue_in(raw).unwrap()
    }

    fn an_entry(id: &str, set: &str, extra: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"{set}\"\nchapter = \"01\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n{extra}\n\n"
        )
    }

    const DRIVEN: &str = "drive = { mode = \"v4\", script = \"selectcert\" }";

    #[test]
    fn the_catalogue_of_the_repository_has_no_complaint() {
        let checks = read_the_catalogue().unwrap();
        assert!(complaints_about(&checks).is_empty());
    }

    #[test]
    fn the_service_set_opens_with_its_greeting() {
        let checks = read_the_catalogue().unwrap();
        assert!(checks
            .iter()
            .find(|check| check.set == "transporte.service")
            .is_some_and(|check| check.greeting));
    }

    #[test]
    fn an_id_repeated_between_two_files_is_named() {
        let one = entries(&an_entry("a_one", "saludo", DRIVEN));
        let other = entries(&an_entry("a_one", "errores", DRIVEN));
        let mixed: Vec<Check> = one.into_iter().chain(other).collect();

        assert_eq!(complaints_about(&mixed), vec!["a_one: id repetido"]);
    }

    #[test]
    fn a_set_outside_the_vocabulary_is_named() {
        let checks = entries(&an_entry("a_one", "inventado", DRIVEN));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: el conjunto «inventado» no existe"]
        );
    }

    #[test]
    fn an_empty_statement_or_citation_is_named() {
        let checks = entries(
            "[[check]]\nid = \"a_one\"\nset = \"errores\"\nchapter = \"15\"\n\
             citation = \"\"\nstatement = \"  \"\nunmeasurable = \"no llega\"\n",
        );
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: «citation» vacío", "a_one: «statement» vacío"]
        );
    }

    #[test]
    fn an_entry_that_is_neither_driven_nor_unmeasurable_is_named() {
        let checks = entries(&an_entry("a_one", "errores", ""));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: ni se conduce ni se declara no medible"]
        );
    }

    #[test]
    fn an_entry_that_names_a_harness_outside_the_registry_is_rejected() {
        let complaint = the_catalogue_in(&an_entry(
            "a_one",
            "errores",
            &format!("{DRIVEN}\nharness = \"an_absent_one\""),
        ))
        .unwrap_err();
        assert!(
            complaint.contains("el arnés «an_absent_one» no existe"),
            "{complaint}"
        );
    }

    #[test]
    fn an_entry_that_names_a_registered_harness_carries_it() {
        let checks = entries(&an_entry(
            "a_one",
            "errores",
            &format!("{DRIVEN}\nharness = \"save_confirmation\""),
        ));
        assert_eq!(
            checks[0].harness.map(|harness| harness.name),
            Some("save_confirmation")
        );
    }

    #[test]
    fn an_unmeasurable_check_that_is_driven_or_unmotivated_is_named() {
        let checks = entries(&format!(
            "{}{}",
            an_entry(
                "a_one",
                "errores",
                &format!("unmeasurable = \"no llega\"\n{DRIVEN}")
            ),
            an_entry("a_two", "errores", "unmeasurable = \"\"")
        ));
        assert_eq!(
            complaints_about(&checks),
            vec![
                "a_one: no medible pero conducida",
                "a_two: no medible sin motivo"
            ]
        );
    }

    #[test]
    fn a_greeting_behind_another_check_or_without_a_drive_is_named() {
        let checks = entries(&format!(
            "{}{}{}",
            an_entry("a_one", "transporte.service", DRIVEN),
            an_entry(
                "a_late_greeting",
                "transporte.service",
                &format!("greeting = true\n{DRIVEN}")
            ),
            an_entry(
                "an_undriven_greeting",
                "errores",
                "unmeasurable = \"Nada que conducir.\"\ngreeting = true"
            )
        ));
        assert_eq!(
            complaints_about(&checks),
            vec![
                "a_late_greeting: saludo que no abre su conjunto",
                "an_undriven_greeting: saludo sin conducir"
            ]
        );
    }

    #[test]
    fn a_check_that_needs_a_person_without_a_question_or_a_warning_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "operaciones",
            &format!("needs = [\"persona\"]\n{DRIVEN}"),
        ));
        assert_eq!(
            complaints_about(&checks),
            vec![
                "a_one: necesita a una persona y no trae pregunta",
                "a_one: necesita a una persona y no trae aviso"
            ]
        );
    }
}
