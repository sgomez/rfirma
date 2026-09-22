//! El catálogo declarativo de la suite de conformidad, leído de `catalogue/`, un fichero por
//! conjunto: los metadatos de cada exigencia, no su cuerpo ejecutable.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use serde::Serialize;
use ts_rs::TS;

use crate::client::Store;
use crate::harness::{the_harness_named, Harness};
use crate::judge::{Code, Contents, Expectation, OnTheWire, Person};
use crate::manifest::{Family, Manifest, Site};

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

/// Qué necesita una comprobación de la persona que está delante, en el orden de sus tramos.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub(crate) enum Assistance {
    #[default]
    None,
    Click,
    Person,
}

impl Assistance {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::None => "ninguna",
            Self::Click => "clic",
            Self::Person => "persona",
        }
    }
}

/// Cómo se conduce al cliente publicado para ejercitar la exigencia.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Drive {
    pub mode: String,
    pub script: String,
}

/// Una exigencia del protocolo con todo lo que se sabe de ella menos cómo se mide.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub set: String,
    pub chapter: String,
    pub citation: String,
    pub statement: String,
    #[serde(default)]
    pub(crate) drive: Option<Drive>,
    #[serde(default, deserialize_with = "a_registered_harness")]
    pub(crate) harness: Option<&'static Harness>,
    #[serde(default)]
    pub(crate) saf: Option<Code>,
    #[serde(default)]
    pub(crate) completes: Option<Contents>,
    /// La condición del manifiesto que juzga la comprobación, con el nombre que le da su guion.
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub no_answer: bool,
    #[serde(default)]
    pub(crate) person: Option<Person>,
    /// Obligatoria en toda comprobación conducida; `None` en las no medibles.
    #[serde(default)]
    pub(crate) assistance: Option<Assistance>,
    #[serde(default)]
    pub(crate) store: Store,
    #[serde(default)]
    pub patience_secs: Option<u64>,
    /// Los puertos que tienen que estar libres antes de conducirla.
    #[serde(default)]
    pub ports: Vec<u16>,
    /// La familia de su guion, que pone el manifiesto al cargar el catálogo.
    #[serde(skip)]
    pub(crate) family: Option<Family>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub question: Option<String>,
    #[serde(default)]
    pub unmeasurable: Option<String>,
    /// Si es el saludo de su familia y su tramo: si no se cumple, no se corre lo que abre.
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
    pub(crate) fn assistance(&self) -> Assistance {
        self.assistance.unwrap_or_default()
    }

    pub(crate) fn needs_a_person(&self) -> bool {
        self.assistance() == Assistance::Person
    }

    pub(crate) fn declared_patience(&self) -> Option<Duration> {
        self.patience_secs.map(Duration::from_secs)
    }

    /// Todo lo que la comprobación dice en prosa y en códigos, sin cómo se conduce: lo que se
    /// cruza con el manual.
    pub fn the_declared_text(&self) -> String {
        [
            Some(self.statement.clone()),
            Some(self.citation.clone()),
            self.saf.as_ref().map(ToString::to_string),
            self.condition.clone(),
            self.warning.clone(),
            self.question.clone(),
            self.unmeasurable.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
    }

    /// Lo que la comprobación espera, tal y como la declara.
    pub(crate) fn expectation(&self) -> Expectation<'_> {
        Expectation {
            on_the_wire: self.what_it_expects_on_the_wire().into_iter().next(),
            person: self.person.as_ref(),
        }
    }

    fn what_it_expects_on_the_wire(&self) -> Vec<OnTheWire<'_>> {
        [
            self.saf.as_ref().map(OnTheWire::Saf),
            self.completes.as_ref().map(OnTheWire::Completes),
            self.condition.as_deref().map(OnTheWire::Condition),
            self.no_answer.then_some(OnTheWire::NoAnswer),
        ]
        .into_iter()
        .flatten()
        .collect()
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
pub fn read_the_catalogue() -> Result<Vec<Check>, String> {
    let manifest = Manifest::of_the_driver()?;
    let mut checks = read_the_catalogue_files()?;
    let complaints = complaints_against(&checks, &manifest);
    if complaints.is_empty() {
        for check in &mut checks {
            check.family = the_family_of(check, &manifest);
        }
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
        malformed_greetings(checks),
        driven_entries_without_an_assistance(checks),
        person_entries_without_a_warning(checks),
        questions_without_a_person(checks),
        expectations_out_of_shape(checks),
    ]
    .concat()
}

fn expectations_out_of_shape(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter_map(|check| {
            let id = &check.id;
            let on_the_wire = check.what_it_expects_on_the_wire().len();
            let expects_something = on_the_wire > 0 || check.person.is_some();
            if check.drive.is_none() {
                return expects_something.then(|| format!("{id}: espera algo sin conducirse"));
            }
            if on_the_wire > 1 {
                Some(format!("{id}: espera más de una cosa del cable"))
            } else if !expects_something {
                Some(format!("{id}: se conduce sin declarar qué espera"))
            } else if check.person.is_some() != check.question.is_some() {
                Some(format!("{id}: la persona y su pregunta no van juntas"))
            } else {
                None
            }
        })
        .collect()
}

fn the_family_of(check: &Check, manifest: &Manifest) -> Option<Family> {
    let drive = check.drive.as_ref()?;
    manifest
        .scripts
        .get(&drive.script)
        .map(|script| script.family)
}

/// Lo que el catálogo cita y el manifiesto no publica —modos, guiones y condiciones— y los saludos
/// que se pisan en su familia y su tramo.
fn complaints_against(checks: &[Check], manifest: &Manifest) -> Vec<String> {
    checks
        .iter()
        .flat_map(|check| complaints_about_the_drive_of(check, manifest))
        .chain(greetings_sharing_a_family_and_a_tranche(checks, manifest))
        .collect()
}

fn greetings_sharing_a_family_and_a_tranche(checks: &[Check], manifest: &Manifest) -> Vec<String> {
    let mut opened = BTreeSet::new();
    checks
        .iter()
        .filter(|check| check.greeting)
        .filter_map(|check| {
            let family = the_family_of(check, manifest)?;
            (!opened.insert((family, check.assistance()))).then(|| {
                format!(
                    "{}: otro saludo abre ya su familia en el tramo {}",
                    check.id,
                    check.assistance().name()
                )
            })
        })
        .collect()
}

fn complaints_about_the_drive_of(check: &Check, manifest: &Manifest) -> Vec<String> {
    let id = &check.id;
    let Some(drive) = &check.drive else {
        return Vec::new();
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
        None if script.site == Site::Handwritten => complaints.push(format!(
            "{id}: el guion a mano «{}» solo informa por condiciones y no espera ninguna",
            drive.script
        )),
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

fn malformed_greetings(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.greeting)
        .filter_map(|check| {
            if check.drive.is_none() {
                Some(format!("{}: saludo sin conducir", check.id))
            } else if check.needs_a_person() {
                Some(format!("{}: saludo que necesita a una persona", check.id))
            } else {
                None
            }
        })
        .collect()
}

fn driven_entries_without_an_assistance(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.drive.is_some() && check.assistance.is_none())
        .map(|check| format!("{}: se conduce sin declarar su asistencia", check.id))
        .collect()
}

fn person_entries_without_a_warning(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.needs_a_person())
        .filter(|check| {
            check
                .warning
                .as_deref()
                .is_none_or(|said| said.trim().is_empty())
        })
        .map(|check| format!("{}: necesita a una persona y no trae aviso", check.id))
        .collect()
}

fn questions_without_a_person(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.question.is_some() && !check.needs_a_person())
        .map(|check| format!("{}: pregunta sin asistencia persona", check.id))
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
saf = "SAF_47"
assistance = "person"
store = "ec"
ports = [63131, 63132]
patience_secs = 90
warning = "Va a aparecer el diálogo del PIN."
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
        assert_eq!(check.saf, Code::try_from("SAF_47".to_owned()).ok());
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
        assert_eq!(check.assistance(), Assistance::Person);
        assert_eq!(check.store, Store::Ec);
        assert_eq!(check.ports, vec![63131, 63132]);
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
        assert_eq!(check.store, Store::Rsa);
        assert!(check.ports.is_empty());
        assert_eq!(check.declared_patience(), None);
    }

    #[test]
    fn a_malformed_catalogue_complains_instead_of_parsing_half() {
        assert!(the_catalogue_in("[[check]]\nid = ").is_err());
    }

    #[test]
    fn the_catalogue_of_the_repository_reads() {
        let checks = read_the_catalogue().unwrap();
        assert_eq!(checks.len(), 161);
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

    const DRIVEN: &str =
        "drive = { mode = \"v4\", script = \"selectcert\" }\nassistance = \"click\"\n\
         completes = {}";

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
    fn a_check_expecting_two_things_on_the_wire_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "errores",
            &format!("{DRIVEN}\nsaf = \"SAF_03\""),
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: espera más de una cosa del cable"]
        );
    }

    #[test]
    fn a_driven_check_that_expects_nothing_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "errores",
            "drive = { mode = \"v4\", script = \"selectcert\" }\nassistance = \"click\"",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: se conduce sin declarar qué espera"]
        );
    }

    #[test]
    fn an_undriven_check_that_expects_something_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "errores",
            "unmeasurable = \"no llega\"\nsaf = \"SAF_03\"",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: espera algo sin conducirse"]
        );
    }

    #[test]
    fn a_person_without_a_question_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "operaciones",
            "drive = { mode = \"v4\", script = \"selectcert\" }\nassistance = \"person\"\n\
             warning = \"Aviso.\"\n\
             person = { yes = \"sí\", no = \"no\", yes_means = \"conforme\" }",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: la persona y su pregunta no van juntas"]
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
    fn every_family_opens_each_of_its_tranches_with_one_greeting() {
        let checks = read_the_catalogue().unwrap();
        let greetings: BTreeSet<(Family, Assistance)> = checks
            .iter()
            .filter(|check| check.greeting)
            .map(|check| (check.family.unwrap(), check.assistance()))
            .collect();
        assert_eq!(
            greetings,
            BTreeSet::from([
                (Family::V4Echo, Assistance::None),
                (Family::Service, Assistance::None),
                (Family::EndToEnd, Assistance::None),
                (Family::EndToEnd, Assistance::Click),
            ])
        );
    }

    #[test]
    fn two_greetings_of_one_family_and_one_tranche_are_named() {
        let checks = entries(&format!(
            "{}{}",
            an_entry("a_one", "saludo", &format!("{DRIVEN}\ngreeting = true")),
            an_entry("a_two", "errores", &format!("{DRIVEN}\ngreeting = true"))
        ));
        assert_eq!(
            complaints_against(&checks, &the_manifest()),
            vec!["a_two: otro saludo abre ya su familia en el tramo clic"]
        );
    }

    #[test]
    fn every_driven_check_takes_the_family_of_its_script() {
        let checks = read_the_catalogue().unwrap();
        assert!(checks
            .iter()
            .all(|check| check.drive.is_some() == check.family.is_some()));
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
            &format!("{DRIVEN}\nharness = \"files_to_load\""),
        ));
        assert_eq!(
            checks[0].harness.map(|harness| harness.name),
            Some("files_to_load")
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
    fn a_greeting_without_a_drive_or_that_needs_a_person_is_named() {
        let checks = entries(&format!(
            "{}{}",
            an_entry(
                "a_person_greeting",
                "transporte.service",
                "drive = { mode = \"v4\", script = \"selectcert\" }\nassistance = \"person\"\n\
                 warning = \"Aviso.\"\ngreeting = true\ncompletes = {}"
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
                "a_person_greeting: saludo que necesita a una persona",
                "an_undriven_greeting: saludo sin conducir"
            ]
        );
    }

    #[test]
    fn a_check_that_needs_a_person_without_a_warning_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "operaciones",
            "drive = { mode = \"v4\", script = \"selectcert\" }\nassistance = \"person\"\n\
             completes = {}",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: necesita a una persona y no trae aviso"]
        );
    }

    #[test]
    fn a_question_without_a_person_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "operaciones",
            &format!("{DRIVEN}\nquestion = \"¿sí? [s/n]\""),
        ));
        assert_eq!(
            complaints_about(&checks),
            vec![
                "a_one: pregunta sin asistencia persona",
                "a_one: la persona y su pregunta no van juntas"
            ]
        );
    }

    #[test]
    fn a_driven_check_without_an_assistance_is_named() {
        let checks = entries(&an_entry(
            "a_one",
            "operaciones",
            "drive = { mode = \"v4\", script = \"selectcert\" }\ncompletes = {}",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_one: se conduce sin declarar su asistencia"]
        );
    }

    #[test]
    fn an_unknown_assistance_store_or_field_is_rejected() {
        for extra in [
            "assistance = \"alguna\"",
            "store = \"rfirma-test-ecc\"",
            "needs = [\"persona\"]",
        ] {
            assert!(
                the_catalogue_in(&an_entry("a_one", "errores", extra)).is_err(),
                "{extra}"
            );
        }
    }
}
