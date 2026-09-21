//! El expediente de una tanda de la suite de conformidad: qué comprobaciones se han corrido, con
//! qué sujeto y desde cuándo.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::baseline::Profile;
use crate::catalogue::Check;

/// El veredicto de una comprobación: uno solo de cada color, y ninguno que pinte de verde un
/// sujeto que se apartó de lo que el protocolo exige.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Compliant,
    Noncompliant,
    NotObservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Pending,
    Resolved(Verdict),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRecord {
    pub suite: String,
    pub chapter: String,
    pub citation: String,
    pub statement: String,
    pub state: CheckState,
    pub date: Option<String>,
    #[serde(default)]
    pub observation: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl CheckRecord {
    fn pending(check: &Check) -> Self {
        Self {
            suite: check.suite.clone(),
            chapter: check.chapter.clone(),
            citation: check.citation.clone(),
            statement: check.statement.clone(),
            state: CheckState::Pending,
            date: None,
            observation: None,
            duration_ms: None,
        }
    }
}

/// Las coordenadas de una tanda, tomadas una sola vez al abrir un expediente nuevo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub os: String,
    pub os_version: String,
    pub subject_version: String,
    pub transport: String,
    pub store: String,
    pub date: String,
}

/// Las coordenadas que da quien abre la tanda, sin la fecha: esa la pone el expediente.
pub struct HeaderCoordinates {
    pub os: String,
    pub os_version: String,
    pub subject_version: String,
    pub transport: String,
    pub store: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Contents {
    subject: String,
    profile: Profile,
    header: Header,
    checks: BTreeMap<String, CheckRecord>,
}

/// El nombre del expediente dentro del directorio de su informe.
pub const THE_DOSSIER_FILE: &str = "dossier.json";

/// El expediente de una tanda: un JSON en la ruta que se le indique, ligado a un único sujeto.
#[derive(Debug)]
pub struct Dossier {
    path: PathBuf,
    contents: Contents,
}

impl Dossier {
    /// Abre el expediente en `path` para `subject`, creándolo con `catalogue` si no existe.
    ///
    /// Rechaza un expediente que otro sujeto o perfil generó en vez de mezclar veredictos. Un
    /// expediente nuevo exige `coordinates`.
    pub fn open(
        path: &Path,
        subject: &str,
        profile: Profile,
        catalogue: &[Check],
        coordinates: Option<HeaderCoordinates>,
    ) -> Result<Self, String> {
        let mut contents = if path.exists() {
            let raw = fs::read_to_string(path)
                .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
            serde_json::from_str(&raw).map_err(|error| {
                format!("{} no es un expediente válido: {error}", path.display())
            })?
        } else {
            let coordinates = coordinates
                .ok_or_else(|| "faltan las coordenadas de la tanda del informe nuevo".to_owned())?;
            Contents {
                subject: subject.to_owned(),
                profile,
                header: Header {
                    os: coordinates.os,
                    os_version: coordinates.os_version,
                    subject_version: coordinates.subject_version,
                    transport: coordinates.transport,
                    store: coordinates.store,
                    date: today(),
                },
                checks: BTreeMap::new(),
            }
        };
        if contents.subject != subject {
            return Err(format!(
                "el expediente {} es de {}, no de {subject}",
                path.display(),
                contents.subject
            ));
        }
        if contents.profile != profile {
            return Err(format!(
                "el expediente {} se abrió con el perfil {}, y esta tanda trae {}",
                path.display(),
                contents.profile.name(),
                profile.name()
            ));
        }
        for check in catalogue {
            contents
                .checks
                .entry(check.id.clone())
                .or_insert_with(|| CheckRecord::pending(check));
        }
        let dossier = Self {
            path: path.to_owned(),
            contents,
        };
        dossier.save()?;
        Ok(dossier)
    }

    pub fn subject(&self) -> &str {
        &self.contents.subject
    }

    pub fn profile(&self) -> Profile {
        self.contents.profile
    }

    /// Lee un expediente ya escrito, sin catálogo ni sujeto con el que cuadrarlo: lo que necesita
    /// quien compara dos tandas.
    pub fn read(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
        let contents = serde_json::from_str(&raw)
            .map_err(|error| format!("{} no es un expediente válido: {error}", path.display()))?;
        Ok(Self {
            path: path.to_owned(),
            contents,
        })
    }

    pub fn header(&self) -> &Header {
        &self.contents.header
    }

    pub fn checks(&self) -> impl Iterator<Item = (&str, &CheckRecord)> {
        self.contents
            .checks
            .iter()
            .map(|(id, record)| (id.as_str(), record))
    }

    pub fn state_of(&self, id: &str) -> Option<CheckState> {
        self.contents.checks.get(id).map(|record| record.state)
    }

    pub fn record_of(&self, id: &str) -> Option<&CheckRecord> {
        self.contents.checks.get(id)
    }

    /// Marca `id` con `verdict` hoy, junto a `observation` si la hubo y lo que tardó, y lo deja
    /// escrito antes de devolver el control.
    pub fn resolve(
        &mut self,
        id: &str,
        verdict: Verdict,
        observation: Option<String>,
        duration: Duration,
    ) -> Result<(), String> {
        if let Some(record) = self.contents.checks.get_mut(id) {
            record.state = CheckState::Resolved(verdict);
            record.date = Some(today());
            record.observation = observation;
            record.duration_ms = Some(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX));
        }
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.contents)
            .map_err(|error| format!("el expediente no se pudo serializar: {error}"))?;
        fs::write(&self.path, json)
            .map_err(|error| format!("{} no se pudo escribir: {error}", self.path.display()))
    }
}

/// La fecha de hoy, `AAAA-MM-DD`: la misma que usa el expediente para fechar un veredicto.
pub fn today() -> String {
    let seconds_since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("el reloj del sistema va después de 1970")
        .as_secs();
    let (year, month, day) = civil_date_from_days((seconds_since_epoch / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Convierte días desde 1970-01-01 en año, mes y día del calendario gregoriano
/// (Howard Hinnant, `https://howardhinnant.github.io/date_algorithms.html`).
fn civil_date_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::the_catalogue_in;

    fn a_catalogue_of(ids: &[&str]) -> Vec<Check> {
        let entries: String = ids
            .iter()
            .map(|id| {
                format!(
                    "[[check]]\nid = \"{id}\"\nsuite = \"errores\"\nchapter = \"15\"\n\
                     citation = \"ProtocolInvocationLauncher.java:741\"\n\
                     statement = \"Algo se rechaza con SAF_03.\"\n\
                     drive = {{ mode = \"v4\", script = \"selectcert\" }}\n\n"
                )
            })
            .collect();
        the_catalogue_in(&entries).unwrap()
    }

    fn some_coordinates() -> HeaderCoordinates {
        HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        }
    }

    #[test]
    fn resolves_a_check_with_the_single_verdict() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let mut dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();

        assert_eq!(
            dossier.state_of("v4_echo_greeting"),
            Some(CheckState::Pending)
        );

        dossier
            .resolve(
                "v4_echo_greeting",
                Verdict::Noncompliant,
                None,
                Duration::ZERO,
            )
            .unwrap();

        assert_eq!(
            dossier.state_of("v4_echo_greeting"),
            Some(CheckState::Resolved(Verdict::Noncompliant))
        );
    }

    #[test]
    fn a_resolved_verdict_its_observation_and_its_duration_survive_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let mut dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();
        dossier
            .resolve(
                "v4_echo_greeting",
                Verdict::Compliant,
                Some("OK".to_owned()),
                Duration::from_millis(1_250),
            )
            .unwrap();

        let reopened =
            Dossier::open(&path, "autofirma", Profile::Autofirma, &catalogue, None).unwrap();

        assert_eq!(
            reopened.state_of("v4_echo_greeting"),
            Some(CheckState::Resolved(Verdict::Compliant))
        );
        let (_, record) = reopened
            .checks()
            .find(|(id, _)| *id == "v4_echo_greeting")
            .unwrap();
        assert_eq!(record.observation.as_deref(), Some("OK"));
        assert_eq!(record.duration_ms, Some(1_250));
        assert_eq!(record.chapter, "15");
        assert_eq!(record.suite, "errores");
    }

    #[test]
    fn a_check_not_yet_run_stays_pending_instead_of_being_omitted() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting", "v3_echo_greeting"]);
        let dossier = Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();

        let ids: Vec<&str> = dossier.checks().map(|(id, _)| id).collect();
        assert!(ids.contains(&"v3_echo_greeting"));
        assert_eq!(
            dossier.state_of("v3_echo_greeting"),
            Some(CheckState::Pending)
        );
    }

    #[test]
    fn a_dossier_of_another_subject_is_refused() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Dossier::open(
            &path,
            "autofirma",
            Profile::Autofirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();

        let complaint =
            Dossier::open(&path, "rfirma", Profile::Rfirma, &catalogue, None).unwrap_err();

        assert!(complaint.contains("es de autofirma, no de rfirma"));
    }

    #[test]
    fn a_dossier_opened_with_another_profile_is_refused_naming_both() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Dossier::open(
            &path,
            "un-binario",
            Profile::Autofirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();

        let complaint =
            Dossier::open(&path, "un-binario", Profile::Rfirma, &catalogue, None).unwrap_err();

        assert!(complaint.contains("perfil autofirma"));
        assert!(complaint.contains("trae rfirma"));
    }

    #[test]
    fn the_profile_of_a_dossier_survives_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Dossier::open(
            &path,
            "un-binario",
            Profile::Rfirma,
            &catalogue,
            Some(some_coordinates()),
        )
        .unwrap();

        assert_eq!(Dossier::read(&path).unwrap().profile(), Profile::Rfirma);
    }
}
