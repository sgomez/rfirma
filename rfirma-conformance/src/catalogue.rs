//! El catálogo declarativo de la suite de conformidad, leído de `catalogue/`, un fichero por
//! conjunto: los metadatos de cada exigencia, no su cuerpo ejecutable.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use crate::harness::{the_harness_named, Harness};
use crate::manifest::{Manifest, Site};

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
    /// La condición del manifiesto que juzga la comprobación, con el nombre que le da su guion.
    #[serde(default)]
    pub condition: Option<String>,
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

/// El catálogo entero validado contra el manifiesto de la sede, o por qué no arranca la suite:
/// cada queja nombra la entrada y lo que le falta.
pub(crate) fn read_the_catalogue() -> Result<Vec<Check>, String> {
    let manifest = Manifest::of_the_driver()?;
    let checks = read_the_catalogue_files()?;
    let complaints = complaints_against(&checks, &manifest);
    if complaints.is_empty() {
        Ok(checks)
    } else {
        Err(format!(
            "el catálogo no casa con el manifiesto de la sede:\n  {}",
            complaints.join("\n  ")
        ))
    }
}

fn read_the_catalogue_files() -> Result<Vec<Check>, String> {
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
        conditions_beside_a_saf(checks),
    ]
    .concat()
}

fn conditions_beside_a_saf(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.condition.is_some() && check.expects_saf.is_some())
        .map(|check| format!("{}: espera a la vez una condición y un SAF", check.id))
        .collect()
}

/// Lo que el catálogo cita y el manifiesto no publica: modos, guiones y condiciones.
fn complaints_against(checks: &[Check], manifest: &Manifest) -> Vec<String> {
    checks
        .iter()
        .flat_map(|check| complaints_about_the_drive_of(check, manifest))
        .collect()
}

fn complaints_about_the_drive_of(check: &Check, manifest: &Manifest) -> Vec<String> {
    let id = &check.id;
    let Some(drive) = &check.drive else {
        return check
            .condition
            .iter()
            .map(|condition| format!("{id}: espera la condición «{condition}» sin conducirse"))
            .collect();
    };
    let mut complaints = Vec::new();
    let mode = manifest.modes.get(&drive.mode);
    match mode {
        None => complaints.push(format!("{id}: el modo «{}» no existe", drive.mode)),
        Some(mode) if mode.bench_only => {
            complaints.push(format!("{id}: el modo «{}» es solo del banco", drive.mode))
        }
        Some(_) => {}
    }
    let Some(script) = manifest.scripts.get(&drive.script) else {
        complaints.push(format!("{id}: el guion «{}» no existe", drive.script));
        return complaints;
    };
    if script.bench_only {
        complaints.push(format!(
            "{id}: el guion «{}» es solo del banco",
            drive.script
        ));
    }
    if mode.is_some() && !script.modes.contains(&drive.mode) {
        complaints.push(format!(
            "{id}: el guion «{}» no corre en el modo «{}»",
            drive.script, drive.mode
        ));
    }
    match &check.condition {
        Some(condition) if !script.conditions.contains(condition) => complaints.push(format!(
            "{id}: el guion «{}» no emite la condición «{condition}»",
            drive.script
        )),
        None if script.site == Site::Handwritten && check.harness.is_none() => {
            complaints.push(format!(
                "{id}: el guion a mano «{}» solo informa por condiciones y no espera ninguna",
                drive.script
            ))
        }
        _ => {}
    }
    complaints
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

    fn the_manifest() -> Manifest {
        Manifest::of_the_driver().unwrap()
    }

    fn complaints_against_the_driver(extra: &str) -> Vec<String> {
        complaints_against(
            &entries(&an_entry("a_one", "errores", extra)),
            &the_manifest(),
        )
    }

    #[test]
    fn the_catalogue_of_the_repository_matches_the_manifest_of_the_driver() {
        let checks = read_the_catalogue_files().unwrap();
        assert_eq!(
            complaints_against(&checks, &the_manifest()),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_script_outside_the_manifest_is_named() {
        assert_eq!(
            complaints_against_the_driver("drive = { mode = \"v4\", script = \"selectcrt\" }"),
            vec!["a_one: el guion «selectcrt» no existe"]
        );
    }

    #[test]
    fn a_mode_outside_the_manifest_is_named() {
        assert_eq!(
            complaints_against_the_driver(
                "drive = { mode = \"bad-uri\", script = \"selectcert\" }"
            ),
            vec!["a_one: el modo «bad-uri» no existe"]
        );
    }

    #[test]
    fn a_condition_its_script_does_not_emit_is_named() {
        assert_eq!(
            complaints_against_the_driver(
                "drive = { mode = \"v4\", script = \"protocol-v4\" }\ncondition = \"the-echo-answers-ok\""
            ),
            vec!["a_one: el guion «protocol-v4» no emite la condición «the-echo-answers-ok»"]
        );
    }

    #[test]
    fn a_script_driven_in_a_mode_it_does_not_run_in_is_named() {
        assert_eq!(
            complaints_against_the_driver(
                "drive = { mode = \"service\", script = \"protocol-v4\" }\n\
                 condition = \"a-candidate-port-bound\""
            ),
            vec!["a_one: el guion «protocol-v4» no corre en el modo «service»"]
        );
    }

    #[test]
    fn a_script_or_a_mode_only_for_the_bench_is_named() {
        assert_eq!(
            complaints_against_the_driver("drive = { mode = \"relay\", script = \"relay\" }"),
            vec![
                "a_one: el modo «relay» es solo del banco",
                "a_one: el guion «relay» es solo del banco"
            ]
        );
    }

    #[test]
    fn a_handwritten_script_without_an_expected_condition_is_named() {
        assert_eq!(
            complaints_against_the_driver("drive = { mode = \"v4\", script = \"protocol-v4\" }"),
            vec!["a_one: el guion a mano «protocol-v4» solo informa por condiciones y no espera ninguna"]
        );
    }

    #[test]
    fn a_check_expecting_a_condition_and_a_saf_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "errores",
            &format!("{DRIVEN}\ncondition = \"a-certificate-alone\"\nexpects_saf = \"SAF_03\""),
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: espera a la vez una condición y un SAF"]
        );
    }

    #[test]
    fn every_script_the_catalogue_does_not_drive_is_marked_for_the_bench_only() {
        let checks = read_the_catalogue_files().unwrap();
        let driven: BTreeSet<&str> = checks
            .iter()
            .filter_map(|check| check.drive.as_ref())
            .map(|drive| drive.script.as_str())
            .collect();
        let manifest = the_manifest();
        let mismarked: Vec<&String> = manifest
            .scripts
            .iter()
            .filter(|(name, script)| script.bench_only == driven.contains(name.as_str()))
            .map(|(name, _)| name)
            .collect();
        assert!(mismarked.is_empty(), "{mismarked:?}");
    }

    #[test]
    fn no_id_of_the_catalogue_is_written_in_the_driver() {
        let driver = crate::errand::the_driver();
        let sources: String = [driver.parent().unwrap().to_path_buf()]
            .iter()
            .flat_map(|dir| javascript_files_under(dir))
            .map(|path| std::fs::read_to_string(path).unwrap())
            .collect();
        let written: Vec<String> = read_the_catalogue_files()
            .unwrap()
            .into_iter()
            .map(|check| check.id)
            .filter(|id| sources.contains(id.as_str()))
            .collect();
        assert!(written.is_empty(), "{written:?}");
    }

    fn javascript_files_under(dir: &std::path::Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .flat_map(|path| {
                if path.is_dir() {
                    javascript_files_under(&path)
                } else if path.extension().is_some_and(|extension| extension == "mjs") {
                    vec![path]
                } else {
                    Vec::new()
                }
            })
            .collect()
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
