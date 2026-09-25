//! El catálogo declarativo de la suite de conformidad, leído de `catalogue/`: sus conjuntos y un
//! fichero por conjunto con los metadatos de cada exigencia, no su cuerpo ejecutable.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer};

use serde::Serialize;
use ts_rs::TS;

use crate::client::Store;
use crate::harness::{the_harness_named, Harness};
use crate::judge::Expectation;
use crate::known_bug::KnownBug;
use crate::label::{the_declared_labels, Label};
use crate::manifest::{Family, Manifest, Site};

/// Un conjunto declarado en `catalogue/sets.toml`: su nombre y sus capítulos, el primero el de
/// omisión.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Set {
    pub name: String,
    pub chapters: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sets {
    set: Vec<Set>,
}

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

/// Qué se exige, dónde lo hace el código de AutoFirma y en qué conjunto y capítulo va.
#[derive(Debug, Clone, Deserialize)]
pub struct Requirement {
    pub set: String,
    /// El de su conjunto si no lo fija; solo lo fija en un conjunto de varios capítulos.
    #[serde(default)]
    pub chapter: String,
    pub citation: String,
    pub statement: String,
}

/// Cómo se provoca el trámite: el guion de la sede y su modo, y lo que el trámite necesita alrededor.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Provocation {
    pub mode: String,
    pub script: String,
    #[serde(default)]
    pub store: Store,
    #[serde(default, deserialize_with = "a_registered_harness")]
    pub harness: Option<&'static Harness>,
    /// Los puertos que tienen que estar libres antes de conducirla.
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub patience_secs: Option<u64>,
}

/// Lo que hace la persona durante el trámite, con la instrucción cerrada que se le da; nunca juzga.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Act {
    #[default]
    None,
    Consent(String),
    Cancel(String),
    PickFile(String),
    SaveAsProposed(String),
    WrongPin(String),
    TypePassword(String),
    MarkArea(String),
    Refuse(String),
}

impl Act {
    pub(crate) fn assistance(&self) -> Assistance {
        match self {
            Self::None => Assistance::None,
            Self::Consent(_) => Assistance::Click,
            _ => Assistance::Person,
        }
    }

    pub(crate) fn instruction(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Consent(said)
            | Self::Cancel(said)
            | Self::PickFile(said)
            | Self::SaveAsProposed(said)
            | Self::WrongPin(said)
            | Self::TypePassword(said)
            | Self::MarkArea(said)
            | Self::Refuse(said) => Some(said),
        }
    }
}

/// Una comprobación que se conduce: cómo se provoca, qué hace la persona y qué se espera.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Trial {
    #[serde(flatten)]
    pub provocation: Provocation,
    #[serde(default)]
    pub act: Act,
    pub expects: Expectation,
    /// Si es el saludo de su familia y su tramo: si no se cumple, no se corre lo que abre.
    #[serde(default)]
    pub greeting: bool,
}

/// Cómo se mide una exigencia: conduciéndola, o no, con el motivo.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Measure {
    Unmeasurable(String),
    Drive(Box<Trial>),
}

/// Una exigencia del protocolo con cómo se provoca y qué se espera de ella.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    #[serde(flatten)]
    pub requirement: Requirement,
    #[serde(flatten)]
    pub(crate) measure: Measure,
    /// Lo que puede explicar su NO CONFORME, igual para cualquier cliente.
    #[serde(
        default,
        rename = "explained_by",
        deserialize_with = "the_declared_labels"
    )]
    pub labels: Vec<Label>,
    /// La familia de su guion, que pone el manifiesto al cargar el catálogo.
    #[serde(skip)]
    pub(crate) family: Option<Family>,
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
    /// La ficha `BUG-NN` por la que AutoFirma 1.9.2 incumple lo que se exige.
    pub fn bug(&self) -> Option<&'static KnownBug> {
        self.labels.iter().find_map(Label::known_bug)
    }

    /// El ADR de rFirma que decide no hacer lo que se exige.
    pub fn adr(&self) -> Option<&str> {
        self.labels.iter().find_map(Label::adr)
    }

    pub(crate) fn trial(&self) -> Option<&Trial> {
        match &self.measure {
            Measure::Drive(trial) => Some(trial),
            Measure::Unmeasurable(_) => None,
        }
    }

    pub(crate) fn unmeasurable(&self) -> Option<&str> {
        match &self.measure {
            Measure::Unmeasurable(motive) => Some(motive),
            Measure::Drive(_) => None,
        }
    }

    pub(crate) fn provocation(&self) -> Option<&Provocation> {
        self.trial().map(|trial| &trial.provocation)
    }

    pub(crate) fn assistance(&self) -> Assistance {
        self.trial()
            .map_or(Assistance::None, |trial| trial.act.assistance())
    }

    pub(crate) fn needs_a_person(&self) -> bool {
        self.assistance() == Assistance::Person
    }

    pub(crate) fn greeting(&self) -> bool {
        self.trial().is_some_and(|trial| trial.greeting)
    }

    pub(crate) fn store(&self) -> Store {
        self.provocation()
            .map_or_else(Store::default, |provocation| provocation.store)
    }

    pub(crate) fn harness(&self) -> Option<&'static Harness> {
        self.provocation()?.harness
    }

    pub(crate) fn ports(&self) -> &[u16] {
        self.provocation()
            .map_or(&[], |provocation| &provocation.ports)
    }

    pub(crate) fn instruction(&self) -> Option<&str> {
        self.trial()?.act.instruction()
    }

    pub(crate) fn declared_patience(&self) -> Option<Duration> {
        self.provocation()?.patience_secs.map(Duration::from_secs)
    }

    /// Todo lo que la comprobación dice en prosa y en códigos, sin cómo se conduce: lo que se
    /// cruza con el manual.
    pub fn the_declared_text(&self) -> String {
        let expects = self.trial().map(|trial| trial.expects.the_declared_text());
        [
            Some(self.requirement.statement.clone()),
            Some(self.requirement.citation.clone()),
            expects.filter(|said| !said.is_empty()),
            self.instruction().map(str::to_owned),
            self.unmeasurable().map(str::to_owned),
            self.bug().map(|bug| bug.id.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
    }
}

/// Dónde vive el catálogo: un directorio con un fichero por conjunto, no un literal empotrado,
/// porque es lo que se lee y se revisa.
pub(crate) fn the_catalogue_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("catalogue")
}

const THE_SETS_FILE: &str = "sets.toml";

fn the_set_file(set: &Set) -> PathBuf {
    the_catalogue_dir().join(format!("{}.toml", set.name))
}

pub(crate) fn the_declared_sets() -> Result<Vec<Set>, String> {
    let path = the_catalogue_dir().join(THE_SETS_FILE);
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    the_sets_in(&raw).map_err(|complaint| format!("{}: {complaint}", path.display()))
}

fn the_sets_in(raw: &str) -> Result<Vec<Set>, String> {
    let sets: Sets =
        toml::from_str(raw).map_err(|error| format!("no son conjuntos válidos: {error}"))?;
    let complaints = complaints_about_the_sets(&sets.set);
    if complaints.is_empty() {
        Ok(sets.set)
    } else {
        Err(complaints.join("; "))
    }
}

fn complaints_about_the_sets(sets: &[Set]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    sets.iter()
        .filter_map(|set| {
            if !seen.insert(set.name.as_str()) {
                Some(format!("el conjunto «{}» está repetido", set.name))
            } else if set.chapters.is_empty() {
                Some(format!("el conjunto «{}» no tiene capítulo", set.name))
            } else {
                None
            }
        })
        .collect()
}

/// Las entradas de un conjunto con su capítulo resuelto, o lo que tienen fuera de su sitio.
fn the_set_in(set: &Set, raw: &str) -> Result<Vec<Check>, String> {
    let mut checks = the_catalogue_in(raw)?;
    let complaints: Vec<String> = checks
        .iter_mut()
        .flat_map(|check| complaints_about_the_place_of(check, set))
        .collect();
    if complaints.is_empty() {
        Ok(checks)
    } else {
        Err(complaints.join("; "))
    }
}

fn complaints_about_the_place_of(check: &mut Check, set: &Set) -> Option<String> {
    let id = &check.id;
    let requirement = &mut check.requirement;
    if requirement.set != set.name {
        return Some(format!(
            "{id}: dice ser del conjunto «{}» y está en el fichero de «{}»",
            requirement.set, set.name
        ));
    }
    if requirement.chapter.is_empty() {
        requirement.chapter = set.chapters[0].clone();
        None
    } else if set.chapters.len() < 2 {
        Some(format!(
            "{id}: fija su capítulo y su conjunto «{}» solo tiene uno",
            set.name
        ))
    } else if !set.chapters.contains(&requirement.chapter) {
        Some(format!(
            "{id}: el capítulo {} no es de su conjunto «{}»",
            requirement.chapter, set.name
        ))
    } else {
        None
    }
}

fn files_of_no_set(sets: &[Set]) -> Result<Vec<String>, String> {
    let dir = the_catalogue_dir();
    let declared: BTreeSet<String> = sets
        .iter()
        .map(|set| format!("{}.toml", set.name))
        .chain([THE_SETS_FILE.to_owned()])
        .collect();
    let mut orphans: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|error| format!("{} no se pudo leer: {error}", dir.display()))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".toml") && !declared.contains(name))
        .map(|name| format!("{name}: no es de ningún conjunto declarado"))
        .collect();
    orphans.sort();
    Ok(orphans)
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
    let sets = the_declared_sets()?;
    let orphans = files_of_no_set(&sets)?;
    if !orphans.is_empty() {
        return Err(format!(
            "el catálogo tiene ficheros de más:\n  {}",
            orphans.join("\n  ")
        ));
    }
    let mut checks = Vec::new();
    for set in &sets {
        let path = the_set_file(set);
        let raw = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
        let entries = the_set_in(set, &raw)
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
        empty_fields(checks),
        greetings_that_need_a_person(checks),
        checks_that_measure_the_same(checks),
    ]
    .concat()
}

fn the_family_of(check: &Check, manifest: &Manifest) -> Option<Family> {
    let provocation = check.provocation()?;
    manifest
        .scripts
        .get(&provocation.script)
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
        .filter(|check| check.greeting())
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
    let Some(trial) = check.trial() else {
        return Vec::new();
    };
    let drive = &trial.provocation;
    let conditions = trial.expects.conditions();
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
            "{id}: el guion «{}» no funciona en el modo «{}»",
            drive.script, drive.mode
        ));
    }
    if conditions.is_empty() && script.site == Site::Handwritten {
        complaints.push(format!(
            "{id}: el guion a mano «{}» solo informa por condiciones y no espera ninguna",
            drive.script
        ));
    }
    complaints.extend(
        conditions
            .iter()
            .filter(|condition| !script.conditions.contains(condition))
            .map(|condition| {
                format!(
                    "{id}: el guion «{}» no emite la condición «{condition}»",
                    drive.script
                )
            }),
    );
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

fn empty_fields(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .flat_map(|check| {
            let requirement = &check.requirement;
            [
                ("id", Some(check.id.as_str())),
                ("set", Some(requirement.set.as_str())),
                ("chapter", Some(requirement.chapter.as_str())),
                ("citation", Some(requirement.citation.as_str())),
                ("statement", Some(requirement.statement.as_str())),
                ("unmeasurable", check.unmeasurable()),
                ("act", check.instruction()),
            ]
            .into_iter()
            .filter(|(_, value)| value.is_some_and(|value| value.trim().is_empty()))
            .map(|(field, _)| format!("{}: «{field}» vacío", check.id))
        })
        .collect()
}

fn greetings_that_need_a_person(checks: &[Check]) -> Vec<String> {
    checks
        .iter()
        .filter(|check| check.greeting() && check.needs_a_person())
        .map(|check| format!("{}: saludo que necesita a una persona", check.id))
        .collect()
}

/// Dos comprobaciones con el mismo trámite, la misma acción y la misma expectativa son una sola.
fn checks_that_measure_the_same(checks: &[Check]) -> Vec<String> {
    let driven: Vec<(&Check, &Trial)> = checks
        .iter()
        .filter_map(|check| check.trial().map(|trial| (check, trial)))
        .collect();
    driven
        .iter()
        .enumerate()
        .filter_map(|(at, (check, trial))| {
            driven[..at]
                .iter()
                .find(|(_, earlier)| measure_the_same(earlier, trial))
                .map(|(earlier, _)| format!("{}: mide lo mismo que {}", check.id, earlier.id))
        })
        .collect()
}

fn measure_the_same(one: &Trial, other: &Trial) -> bool {
    let (this, that) = (&one.provocation, &other.provocation);
    this.mode == that.mode
        && this.script == that.script
        && this.store == that.store
        && this.harness.map(|harness| harness.name) == that.harness.map(|harness| harness.name)
        && std::mem::discriminant(&one.act) == std::mem::discriminant(&other.act)
        && one.expects.is_the_same_as(&other.expects)
}

pub(crate) fn the_catalogue_in(raw: &str) -> Result<Vec<Check>, String> {
    let catalogue: Catalogue =
        toml::from_str(raw).map_err(|error| format!("no es un catálogo válido: {error}"))?;
    Ok(catalogue
        .check
        .into_iter()
        .map(|mut check| {
            check.requirement.statement = as_one_line(&check.requirement.statement);
            check
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

    const AN_ENTRY_HEAD: &str = r#"
[[check]]
id = "an_origin_that_is_not_local_is_rejected"
set = "transporte.websocket"
chapter = "05"
citation = "AfirmaWebSocketServerV4.java:57-68"
statement = """
El canal responde SAF_47 a cualquier origen que no sea 127.0.0.1.
"""
"#;

    const AN_ENTRY_DRIVE: &str = r#"
[check.drive]
mode = "v4"
script = "selectcert"
store = "ec"
ports = [63131, 63132]
patience_secs = 90
act.wrong_pin = "Va a aparecer el diálogo del PIN: teclea 0000."
expects.code = "SAF_47"
"#;

    fn an_entry_with(extra: &str) -> String {
        format!("{AN_ENTRY_HEAD}{extra}\n{AN_ENTRY_DRIVE}")
    }

    fn the_entry() -> Check {
        the_catalogue_in(&an_entry_with("")).unwrap().remove(0)
    }

    #[test]
    fn reads_an_entry_with_every_field() {
        let check = the_entry();
        assert_eq!(check.id, "an_origin_that_is_not_local_is_rejected");
        assert_eq!(check.requirement.set, "transporte.websocket");
        assert_eq!(check.requirement.chapter, "05");
        let trial = check.trial().unwrap();
        assert_eq!(trial.provocation.mode, "v4");
        assert_eq!(trial.provocation.script, "selectcert");
        assert_eq!(
            trial.act,
            Act::WrongPin("Va a aparecer el diálogo del PIN: teclea 0000.".to_owned())
        );
        assert_eq!(trial.expects.the_declared_text(), "SAF_47");
    }

    #[test]
    fn the_statement_arrives_without_the_newlines_of_its_block() {
        assert_eq!(
            the_entry().requirement.statement,
            "El canal responde SAF_47 a cualquier origen que no sea 127.0.0.1."
        );
    }

    #[test]
    fn reads_what_a_check_needs_from_its_declaration() {
        let check = the_entry();
        assert_eq!(check.assistance(), Assistance::Person);
        assert_eq!(check.store(), Store::Ec);
        assert_eq!(check.ports(), [63131, 63132]);
        assert_eq!(check.declared_patience(), Some(Duration::from_secs(90)));
        assert_eq!(
            check.instruction(),
            Some("Va a aparecer el diálogo del PIN: teclea 0000.")
        );
    }

    #[test]
    fn a_check_that_needs_nothing_says_so() {
        let checks =
            the_catalogue_in(&an_entry("an_id", "errores", "expects.code = \"SAF_03\"")).unwrap();
        let check = &checks[0];
        assert_eq!(check.assistance(), Assistance::None);
        assert_eq!(check.instruction(), None);
        assert_eq!(check.store(), Store::Rsa);
        assert!(check.ports().is_empty());
        assert_eq!(check.declared_patience(), None);
    }

    #[test]
    fn the_assistance_follows_from_what_the_person_does() {
        let assistance_of = |act: &str| {
            the_catalogue_in(&an_entry("an_id", "errores", &format!("{act}\n{EXPECTS}")))
                .unwrap()
                .remove(0)
                .assistance()
        };
        assert_eq!(assistance_of(""), Assistance::None);
        assert_eq!(assistance_of("act.consent = \"Elige.\""), Assistance::Click);
        for named in [
            "cancel",
            "pick_file",
            "save_as_proposed",
            "wrong_pin",
            "type_password",
            "mark_area",
            "refuse",
        ] {
            assert_eq!(
                assistance_of(&format!("act.{named} = \"Haz esto.\"")),
                Assistance::Person,
                "{named}"
            );
        }
    }

    #[test]
    fn an_unmeasurable_check_is_not_driven_and_needs_nobody() {
        let checks = the_catalogue_in(&an_unmeasurable_entry("an_id", "no llega")).unwrap();
        assert_eq!(checks[0].unmeasurable(), Some("no llega"));
        assert!(checks[0].trial().is_none());
        assert_eq!(checks[0].assistance(), Assistance::None);
    }

    #[test]
    fn a_shape_the_types_do_not_admit_is_rejected_when_read() {
        for entry in [
            an_entry(
                "a",
                "errores",
                "expects = { code = \"SAF_03\", completes = {} }",
            ),
            an_entry("a", "errores", ""),
            an_entry("a", "errores", &format!("act = \"consent\"\n{EXPECTS}")),
            an_entry("a", "errores", &format!("act.watch = \"Mira.\"\n{EXPECTS}")),
            an_entry("a", "errores", &format!("question = \"¿Sí?\"\n{EXPECTS}")),
            format!(
                "{}unmeasurable = \"no llega\"\n",
                an_entry("a", "errores", EXPECTS)
            ),
            format!(
                "{}greeting = true\n",
                an_unmeasurable_entry("a", "no llega")
            ),
        ] {
            assert!(the_catalogue_in(&entry).is_err(), "{entry}");
        }
    }

    #[test]
    fn a_malformed_catalogue_complains_instead_of_parsing_half() {
        assert!(the_catalogue_in("[[check]]\nid = ").is_err());
    }

    fn the_labels_of(explained_by: &str) -> Result<Vec<String>, String> {
        let checks = the_catalogue_in(&an_entry_with(explained_by))?;
        Ok(checks[0].labels.iter().map(Label::name).collect())
    }

    #[test]
    fn a_bug_still_present_in_master_brings_the_label_of_master_along() {
        let declared = "explained_by.autofirma = \"BUG-15\"";
        let check = the_catalogue_in(&an_entry_with(declared))
            .unwrap()
            .remove(0);

        assert_eq!(check.bug().map(|bug| bug.id.as_str()), Some("BUG-15"));
        assert!(check.the_declared_text().contains("BUG-15"));
        assert_eq!(
            the_labels_of(declared).unwrap(),
            ["autofirma:bug:1.9.2", "autofirma:bug:master"]
        );
    }

    #[test]
    fn a_bug_fixed_in_master_brings_only_the_label_of_1_9_2() {
        assert_eq!(
            the_labels_of("explained_by.autofirma = \"BUG-18\"").unwrap(),
            ["autofirma:bug:1.9.2"]
        );
    }

    #[test]
    fn the_label_of_master_cannot_be_written_by_hand() {
        assert!(the_labels_of("explained_by.master = \"BUG-15\"").is_err());
    }

    #[test]
    fn a_bug_outside_the_registry_is_refused() {
        let complaint = the_labels_of("explained_by.autofirma = \"BUG-99\"").unwrap_err();

        assert!(
            complaint.contains("BUG-99 no está en el registro de bugs"),
            "{complaint}"
        );
    }

    #[test]
    fn a_deliberate_deviation_is_labelled_by_the_number_of_its_adr() {
        let declared = "explained_by.rfirma = \"ADR-0010\"";

        assert_eq!(the_labels_of(declared).unwrap(), ["rfirma:adr-0010"]);
        assert_eq!(
            the_catalogue_in(&an_entry_with(declared)).unwrap()[0].adr(),
            Some("ADR-0010")
        );
    }

    #[test]
    fn an_adr_label_that_is_not_adr_nnnn_is_refused() {
        for written in ["0010", "ADR-10", "adr-0010", "ADR-00100"] {
            let complaint =
                the_labels_of(&format!("explained_by.rfirma = \"{written}\"")).unwrap_err();
            assert!(complaint.contains("se escribe ADR-NNNN"), "{complaint}");
        }
    }

    #[test]
    fn a_deprecated_format_is_labelled_by_the_manual_and_the_rest_are_not() {
        assert_eq!(
            the_labels_of("explained_by.manual = \"deprecated\"").unwrap(),
            ["manual:deprecated"]
        );
        assert!(the_entry().labels.is_empty());
    }

    #[test]
    fn a_check_may_carry_several_labels_in_a_fixed_order() {
        assert_eq!(
            the_labels_of("explained_by = { manual = \"deprecated\", rfirma = \"ADR-0023\" }")
                .unwrap(),
            ["rfirma:adr-0023", "manual:deprecated"]
        );
    }

    #[test]
    fn a_label_outside_the_closed_list_is_refused() {
        for written in [
            "explained_by.manual = \"obsolete\"",
            "explained_by.referee = \"x\"",
            "bug = \"BUG-15\"",
            "deprecated = true",
        ] {
            assert!(the_labels_of(written).is_err(), "{written}");
        }
    }

    #[test]
    fn every_check_of_the_repository_takes_a_chapter_of_its_declared_set() {
        let sets = the_declared_sets().unwrap();
        let misplaced: Vec<String> = read_the_catalogue()
            .unwrap()
            .into_iter()
            .filter(|check| {
                !sets.iter().any(|set| {
                    set.name == check.requirement.set
                        && set.chapters.contains(&check.requirement.chapter)
                })
            })
            .map(|check| check.id)
            .collect();
        assert!(misplaced.is_empty(), "{misplaced:?}");
    }

    #[test]
    fn the_catalogue_orders_its_blocks_like_its_declared_sets() {
        let checks = read_the_catalogue().unwrap();
        let mut blocks: Vec<&str> = Vec::new();
        for check in &checks {
            if blocks.last() != Some(&check.requirement.set.as_str()) {
                blocks.push(&check.requirement.set);
            }
        }
        let declared: Vec<String> = the_declared_sets()
            .unwrap()
            .into_iter()
            .map(|set| set.name)
            .filter(|name| blocks.contains(&name.as_str()))
            .collect();
        assert_eq!(blocks, declared);
    }

    fn a_set(name: &str, chapters: &[&str]) -> Set {
        Set {
            name: name.to_owned(),
            chapters: chapters
                .iter()
                .map(|chapter| (*chapter).to_owned())
                .collect(),
        }
    }

    const EXPECTS: &str = "expects.completes = {}";

    fn an_entry_without_a_chapter(id: &str, set: &str, extra: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"{set}\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n{extra}\n\
             unmeasurable = \"Nada que conducir.\"\n\n"
        )
    }

    fn an_unmeasurable_entry(id: &str, motive: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"errores\"\nchapter = \"15\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\nunmeasurable = \"{motive}\"\n\n"
        )
    }

    #[test]
    fn a_check_without_a_chapter_takes_the_first_of_its_set() {
        let checks = the_set_in(
            &a_set("firma", &["06", "11", "12"]),
            &an_entry_without_a_chapter("a_one", "firma", ""),
        )
        .unwrap();
        assert_eq!(checks[0].requirement.chapter, "06");
    }

    #[test]
    fn a_check_of_a_set_with_several_chapters_may_fix_one_of_them() {
        let checks = the_set_in(
            &a_set("firma", &["06", "11", "12"]),
            &an_entry_without_a_chapter("a_one", "firma", "chapter = \"12\""),
        )
        .unwrap();
        assert_eq!(checks[0].requirement.chapter, "12");
    }

    #[test]
    fn a_check_out_of_the_place_of_its_file_is_named() {
        let complaint = the_set_in(
            &a_set("firma", &["06", "11", "12"]),
            &format!(
                "{}{}{}",
                an_entry_without_a_chapter("a_one", "lote", ""),
                an_entry_without_a_chapter("a_two", "firma", "chapter = \"08\""),
                an_entry_without_a_chapter("a_three", "firma", "chapter = \"06\"")
            ),
        )
        .unwrap_err();
        assert_eq!(
            complaint,
            "a_one: dice ser del conjunto «lote» y está en el fichero de «firma»; \
             a_two: el capítulo 08 no es de su conjunto «firma»"
        );
    }

    #[test]
    fn a_check_that_fixes_the_chapter_of_a_set_with_only_one_is_named() {
        let complaint = the_set_in(
            &a_set("lote", &["08"]),
            &an_entry_without_a_chapter("a_one", "lote", "chapter = \"08\""),
        )
        .unwrap_err();
        assert_eq!(
            complaint,
            "a_one: fija su capítulo y su conjunto «lote» solo tiene uno"
        );
    }

    #[test]
    fn a_repeated_set_or_one_without_a_chapter_is_named() {
        let complaint = the_sets_in(
            "[[set]]\nname = \"lote\"\nchapters = [\"08\"]\n\n\
             [[set]]\nname = \"lote\"\nchapters = [\"08\"]\n\n\
             [[set]]\nname = \"firma\"\nchapters = []\n",
        )
        .unwrap_err();
        assert_eq!(
            complaint,
            "el conjunto «lote» está repetido; el conjunto «firma» no tiene capítulo"
        );
    }

    #[test]
    fn every_file_of_the_catalogue_belongs_to_a_declared_set() {
        assert_eq!(
            files_of_no_set(&the_declared_sets().unwrap()).unwrap(),
            Vec::<String>::new()
        );
    }

    fn entries(raw: &str) -> Vec<Check> {
        the_catalogue_in(raw).unwrap()
    }

    fn an_entry(id: &str, set: &str, drive: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"{set}\"\nchapter = \"01\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n\n\
             [check.drive]\nmode = \"v4\"\nscript = \"selectcert\"\n{drive}\n\n"
        )
    }

    fn a_drive(mode: &str, script: &str, rest: &str) -> String {
        format!(
            "[[check]]\nid = \"a_one\"\nset = \"errores\"\nchapter = \"01\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n\n\
             [check.drive]\nmode = \"{mode}\"\nscript = \"{script}\"\n{rest}\n"
        )
    }

    const CLICKED: &str = "act.consent = \"Elige.\"\nexpects.completes = {}";

    #[test]
    fn the_catalogue_of_the_repository_has_no_complaint() {
        let checks = read_the_catalogue().unwrap();
        assert_eq!(complaints_about(&checks), Vec::<String>::new());
    }

    fn the_manifest() -> Manifest {
        Manifest::of_the_driver().unwrap()
    }

    fn complaints_against_the_driver(mode: &str, script: &str, rest: &str) -> Vec<String> {
        complaints_against(&entries(&a_drive(mode, script, rest)), &the_manifest())
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
            complaints_against_the_driver("v4", "selectcrt", EXPECTS),
            vec!["a_one: el guion «selectcrt» no existe"]
        );
    }

    #[test]
    fn a_mode_outside_the_manifest_is_named() {
        assert_eq!(
            complaints_against_the_driver("bad-uri", "selectcert", EXPECTS),
            vec!["a_one: el modo «bad-uri» no existe"]
        );
    }

    #[test]
    fn a_condition_its_script_does_not_emit_is_named() {
        assert_eq!(
            complaints_against_the_driver(
                "v4",
                "protocol-v4",
                "expects.completes.conditions = [\"the-echo-answers-ok\"]"
            ),
            vec!["a_one: el guion «protocol-v4» no emite la condición «the-echo-answers-ok»"]
        );
    }

    #[test]
    fn only_the_conditions_of_a_list_its_script_does_not_emit_are_named() {
        assert_eq!(
            complaints_against_the_driver(
                "v4",
                "protocol-v4",
                "expects.completes.conditions = [\"a-candidate-port-bound\", \"the-echo-answers-ok\"]"
            ),
            vec!["a_one: el guion «protocol-v4» no emite la condición «the-echo-answers-ok»"]
        );
    }

    #[test]
    fn a_script_driven_in_a_mode_it_does_not_run_in_is_named() {
        assert_eq!(
            complaints_against_the_driver(
                "service",
                "protocol-v4",
                "expects.completes.conditions = [\"a-candidate-port-bound\"]"
            ),
            vec!["a_one: el guion «protocol-v4» no funciona en el modo «service»"]
        );
    }

    #[test]
    fn a_script_or_a_mode_only_for_the_bench_is_named() {
        let manifest = Manifest::from_json(
            r#"{"modes":{"banco":{"bench_only":true}},"scripts":{"guion":{"site":"published",
            "family":"end-to-end","modes":["banco"],"conditions":[],"bench_only":true}}}"#,
        )
        .unwrap();
        assert_eq!(
            complaints_against(&entries(&a_drive("banco", "guion", EXPECTS)), &manifest),
            vec![
                "a_one: el modo «banco» es solo del banco",
                "a_one: el guion «guion» es solo del banco"
            ]
        );
    }

    #[test]
    fn a_handwritten_script_without_an_expected_condition_is_named() {
        assert_eq!(
            complaints_against_the_driver("v4", "protocol-v4", EXPECTS),
            vec!["a_one: el guion a mano «protocol-v4» solo informa por condiciones y no espera ninguna"]
        );
    }

    #[test]
    fn every_script_the_catalogue_does_not_drive_is_marked_for_the_bench_only() {
        let checks = read_the_catalogue_files().unwrap();
        let driven: BTreeSet<&str> = checks
            .iter()
            .filter_map(|check| check.provocation())
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
            .filter(|check| check.greeting())
            .map(|check| (check.family.unwrap(), check.assistance()))
            .collect();
        assert_eq!(
            greetings,
            BTreeSet::from([
                (Family::V4Echo, Assistance::None),
                (Family::Service, Assistance::None),
                (Family::EndToEnd, Assistance::None),
                (Family::EndToEnd, Assistance::Click),
                (Family::Intermediate, Assistance::Click),
            ])
        );
    }

    #[test]
    fn two_greetings_of_one_family_and_one_tranche_are_named() {
        let checks = entries(&format!(
            "{}{}",
            an_entry("a_one", "saludo", &format!("{CLICKED}\ngreeting = true")),
            an_entry("a_two", "errores", &format!("{CLICKED}\ngreeting = true"))
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
            .all(|check| check.trial().is_some() == check.family.is_some()));
    }

    #[test]
    fn an_id_repeated_between_two_files_is_named() {
        let one = entries(&an_entry("a_one", "saludo", CLICKED));
        let other = entries(&an_unmeasurable_entry("a_one", "no llega"));
        let mixed: Vec<Check> = one.into_iter().chain(other).collect();

        assert_eq!(complaints_about(&mixed), vec!["a_one: id repetido"]);
    }

    #[test]
    fn an_empty_statement_citation_motive_or_instruction_is_named() {
        let checks = entries(&format!(
            "{}{}",
            "[[check]]\nid = \"a_one\"\nset = \"errores\"\nchapter = \"15\"\n\
             citation = \"\"\nstatement = \"  \"\nunmeasurable = \" \"\n\n",
            an_entry(
                "a_two",
                "errores",
                "act.cancel = \"\"\nexpects.code = \"CANCEL\""
            )
        ));
        assert_eq!(
            complaints_about(&checks),
            vec![
                "a_one: «citation» vacío",
                "a_one: «statement» vacío",
                "a_one: «unmeasurable» vacío",
                "a_two: «act» vacío"
            ]
        );
    }

    #[test]
    fn an_entry_that_names_a_harness_outside_the_registry_is_rejected() {
        let complaint = the_catalogue_in(&an_entry(
            "a_one",
            "errores",
            &format!("harness = \"an_absent_one\"\n{CLICKED}"),
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
            &format!("harness = \"files_to_load\"\n{CLICKED}"),
        ));
        assert_eq!(
            checks[0].harness().map(|harness| harness.name),
            Some("files_to_load")
        );
    }

    #[test]
    fn a_greeting_that_needs_a_person_is_named() {
        let checks = entries(&an_entry(
            "a_person_greeting",
            "transporte.service",
            "act.cancel = \"Cancela.\"\ngreeting = true\nexpects.code = \"CANCEL\"",
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_person_greeting: saludo que necesita a una persona"]
        );
    }

    #[test]
    fn two_checks_that_drive_one_errand_the_same_way_and_expect_the_same_are_named() {
        let checks = entries(&format!(
            "{}{}",
            an_entry(
                "a_one",
                "errores",
                "expects.completes.conditions = [\"a\", \"b\"]"
            ),
            an_entry(
                "a_two",
                "firma",
                "expects.completes.conditions = [\"b\", \"a\"]"
            )
        ));
        assert_eq!(
            complaints_about(&checks),
            vec!["a_two: mide lo mismo que a_one"]
        );
    }

    #[test]
    fn two_checks_of_one_errand_that_differ_in_what_they_expect_the_act_or_the_store_are_not() {
        let checks = entries(&format!(
            "{}{}{}{}",
            an_entry(
                "a_one",
                "errores",
                "act.cancel = \"Cancela.\"\nexpects.code = \"CANCEL\""
            ),
            an_entry(
                "a_two",
                "errores",
                "act.refuse = \"Rechaza.\"\nexpects.code = \"CANCEL\""
            ),
            an_entry(
                "a_three",
                "errores",
                "act.cancel = \"Cancela.\"\nexpects.code = \"SAF_43\""
            ),
            an_entry(
                "a_four",
                "errores",
                "store = \"ec\"\nact.cancel = \"Cancela.\"\nexpects.code = \"CANCEL\""
            ),
        ));
        assert_eq!(complaints_about(&checks), Vec::<String>::new());
    }

    #[test]
    fn an_unknown_act_store_or_field_is_rejected() {
        for extra in [
            "act.alguna = \"Algo.\"",
            "store = \"rfirma-test-ecc\"",
            "needs = [\"persona\"]",
        ] {
            assert!(
                the_catalogue_in(&an_entry(
                    "a_one",
                    "errores",
                    &format!("{extra}\n{EXPECTS}")
                ))
                .is_err(),
                "{extra}"
            );
        }
    }
}
