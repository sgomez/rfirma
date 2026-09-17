//! El expediente de una tanda del sondeo: qué casos se han pasado, con qué sujeto y desde cuándo.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// El veredicto de un caso: tres valores, no dos. Un caso que no reproduce no falla la tanda,
/// emite `Refuted` con sus coordenadas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Confirmed,
    Refuted,
    NotObservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseState {
    Pending,
    Resolved(Verdict),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRecord {
    pub state: CaseState,
    pub date: Option<String>,
}

impl CaseRecord {
    fn pending() -> Self {
        Self {
            state: CaseState::Pending,
            date: None,
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
    header: Header,
    cases: BTreeMap<String, CaseRecord>,
}

/// El expediente de una tanda: un JSON en la ruta que se le indique, ligado a un único sujeto.
pub struct Dossier {
    path: PathBuf,
    contents: Contents,
}

impl Dossier {
    /// Abre el expediente en `path` para `subject`, creándolo con `known_cases` si no existe.
    ///
    /// Rechaza un expediente que otro sujeto generó, en vez de mezclar sus veredictos. Un
    /// expediente nuevo exige `coordinates`: sin ellas no hay tanda que abrir.
    pub fn open(
        path: &Path,
        subject: &str,
        known_cases: &[&str],
        coordinates: Option<HeaderCoordinates>,
    ) -> Result<Self, String> {
        let mut contents = if path.exists() {
            let raw = fs::read_to_string(path)
                .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
            serde_json::from_str(&raw).map_err(|error| {
                format!("{} no es un expediente válido: {error}", path.display())
            })?
        } else {
            let coordinates = coordinates.ok_or_else(|| {
                "faltan las coordenadas de la tanda: --os, --os-version, --subject-version y \
                 --store son obligatorias al abrir un expediente nuevo"
                    .to_owned()
            })?;
            Contents {
                subject: subject.to_owned(),
                header: Header {
                    os: coordinates.os,
                    os_version: coordinates.os_version,
                    subject_version: coordinates.subject_version,
                    transport: coordinates.transport,
                    store: coordinates.store,
                    date: today(),
                },
                cases: BTreeMap::new(),
            }
        };
        if contents.subject != subject {
            return Err(format!(
                "el expediente {} es de {}, no de {subject}",
                path.display(),
                contents.subject
            ));
        }
        for case in known_cases {
            contents
                .cases
                .entry((*case).to_owned())
                .or_insert_with(CaseRecord::pending);
        }
        let dossier = Self {
            path: path.to_owned(),
            contents,
        };
        dossier.save()?;
        Ok(dossier)
    }

    pub fn header(&self) -> &Header {
        &self.contents.header
    }

    pub fn cases(&self) -> impl Iterator<Item = (&str, &CaseRecord)> {
        self.contents
            .cases
            .iter()
            .map(|(name, record)| (name.as_str(), record))
    }

    pub fn state_of(&self, case: &str) -> Option<CaseState> {
        self.contents.cases.get(case).map(|record| record.state)
    }

    /// Marca `case` con `verdict` hoy y lo deja escrito antes de devolver el control.
    pub fn resolve(&mut self, case: &str, verdict: Verdict) -> Result<(), String> {
        self.contents.cases.insert(
            case.to_owned(),
            CaseRecord {
                state: CaseState::Resolved(verdict),
                date: Some(today()),
            },
        );
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

    #[test]
    fn resolves_a_case_with_a_three_valued_verdict() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let mut dossier =
            Dossier::open(&path, "autofirma", &["saludo"], Some(coordinates)).unwrap();

        assert_eq!(dossier.state_of("saludo"), Some(CaseState::Pending));

        dossier.resolve("saludo", Verdict::Refuted).unwrap();

        assert_eq!(
            dossier.state_of("saludo"),
            Some(CaseState::Resolved(Verdict::Refuted))
        );
    }

    #[test]
    fn a_resolved_verdict_survives_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let mut dossier =
            Dossier::open(&path, "autofirma", &["saludo"], Some(coordinates)).unwrap();
        dossier.resolve("saludo", Verdict::Confirmed).unwrap();

        let reopened = Dossier::open(&path, "autofirma", &["saludo"], None).unwrap();

        assert_eq!(
            reopened.state_of("saludo"),
            Some(CaseState::Resolved(Verdict::Confirmed))
        );
    }

    #[test]
    fn a_case_not_yet_run_stays_pending_instead_of_being_omitted() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let dossier = Dossier::open(
            &path,
            "autofirma",
            &["saludo", "tramite"],
            Some(coordinates),
        )
        .unwrap();

        let names: Vec<&str> = dossier.cases().map(|(name, _)| name).collect();
        assert!(names.contains(&"tramite"));
        assert_eq!(dossier.state_of("tramite"), Some(CaseState::Pending));
    }
}
