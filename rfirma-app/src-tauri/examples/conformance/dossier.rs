//! El expediente de una tanda de la suite de conformidad: qué casos se han pasado, con qué
//! sujeto y desde cuándo.

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
    /// Lo que la persona dijo haber visto, en un caso interactivo. Ausente en un caso automático.
    #[serde(default)]
    pub observation: Option<String>,
}

impl CaseRecord {
    fn pending() -> Self {
        Self {
            state: CaseState::Pending,
            date: None,
            observation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolVerdict {
    Compliant,
    Discrepant,
    NotObservable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolState {
    Pending,
    Resolved(ProtocolVerdict),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolRecord {
    pub chapter: String,
    pub citation: String,
    pub statement: String,
    pub state: ProtocolState,
    pub date: Option<String>,
    #[serde(default)]
    pub observation: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProtocolConditionDefinition {
    pub id: &'static str,
    pub chapter: &'static str,
    pub citation: &'static str,
    pub statement: &'static str,
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
    #[serde(default)]
    protocol: BTreeMap<String, ProtocolRecord>,
}

/// El expediente de una tanda: un JSON en la ruta que se le indique, ligado a un único sujeto.
pub struct Dossier {
    path: PathBuf,
    contents: Contents,
}

impl Dossier {
    /// Abre el expediente en `path` para `subject`, creándolo con `known_cases` y `known_conditions` si no existe.
    ///
    /// Rechaza un expediente que otro sujeto generó, en vez de mezclar sus veredictos. Un
    /// expediente nuevo exige `coordinates`: sin ellas no hay tanda que abrir.
    pub fn open(
        path: &Path,
        subject: &str,
        known_cases: &[&str],
        known_conditions: &[ProtocolConditionDefinition],
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
                protocol: BTreeMap::new(),
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
        for condition in known_conditions {
            contents
                .protocol
                .entry(condition.id.to_owned())
                .or_insert_with(|| ProtocolRecord {
                    chapter: condition.chapter.to_owned(),
                    citation: condition.citation.to_owned(),
                    statement: condition.statement.to_owned(),
                    state: ProtocolState::Pending,
                    date: None,
                    observation: None,
                });
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

    pub fn protocol_conditions(&self) -> impl Iterator<Item = (&str, &ProtocolRecord)> {
        self.contents
            .protocol
            .iter()
            .map(|(id, record)| (id.as_str(), record))
    }

    pub fn protocol_state_of(&self, id: &str) -> Option<ProtocolState> {
        self.contents.protocol.get(id).map(|record| record.state)
    }

    /// Marca `case` con `verdict` hoy, junto a `observation` si la hubo, y lo deja escrito antes
    /// de devolver el control.
    pub fn resolve(
        &mut self,
        case: &str,
        verdict: Verdict,
        observation: Option<String>,
    ) -> Result<(), String> {
        self.contents.cases.insert(
            case.to_owned(),
            CaseRecord {
                state: CaseState::Resolved(verdict),
                date: Some(today()),
                observation,
            },
        );
        self.save()
    }

    /// Marca `id` con `verdict` hoy, junto a `observation` si la hubo, y lo deja escrito antes
    /// de devolver el control.
    pub fn resolve_protocol(
        &mut self,
        id: &str,
        verdict: ProtocolVerdict,
        observation: Option<String>,
    ) -> Result<(), String> {
        if let Some(record) = self.contents.protocol.get_mut(id) {
            record.state = ProtocolState::Resolved(verdict);
            record.date = Some(today());
            record.observation = observation;
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
            Dossier::open(&path, "autofirma", &["saludo"], &[], Some(coordinates)).unwrap();

        assert_eq!(dossier.state_of("saludo"), Some(CaseState::Pending));

        dossier.resolve("saludo", Verdict::Refuted, None).unwrap();

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
            Dossier::open(&path, "autofirma", &["saludo"], &[], Some(coordinates)).unwrap();
        dossier.resolve("saludo", Verdict::Confirmed, None).unwrap();

        let reopened = Dossier::open(&path, "autofirma", &["saludo"], &[], None).unwrap();

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
            &[],
            Some(coordinates),
        )
        .unwrap();

        let names: Vec<&str> = dossier.cases().map(|(name, _)| name).collect();
        assert!(names.contains(&"tramite"));
        assert_eq!(dossier.state_of("tramite"), Some(CaseState::Pending));
    }

    #[test]
    fn an_observation_survives_a_reopen_alongside_the_verdict() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let mut dossier =
            Dossier::open(&path, "autofirma", &["saludo"], &[], Some(coordinates)).unwrap();
        dossier
            .resolve(
                "saludo",
                Verdict::Confirmed,
                Some("se pidió elegir certificado".to_owned()),
            )
            .unwrap();

        let reopened = Dossier::open(&path, "autofirma", &["saludo"], &[], None).unwrap();

        let (_, record) = reopened
            .cases()
            .find(|(name, _)| *name == "saludo")
            .unwrap();
        assert_eq!(
            record.observation.as_deref(),
            Some("se pidió elegir certificado")
        );
    }

    #[test]
    fn resolves_a_protocol_condition_with_a_three_valued_verdict() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let condition = ProtocolConditionDefinition {
            id: "v4_echo_greeting",
            chapter: "05",
            citation: "AfirmaWebSocketServerV4.java:81-83",
            statement: "La petición de eco recibe OK",
        };
        let mut dossier =
            Dossier::open(&path, "autofirma", &[], &[condition], Some(coordinates)).unwrap();

        assert_eq!(
            dossier.protocol_state_of("v4_echo_greeting"),
            Some(ProtocolState::Pending)
        );

        dossier
            .resolve_protocol(
                "v4_echo_greeting",
                ProtocolVerdict::Compliant,
                Some("OK".to_owned()),
            )
            .unwrap();

        assert_eq!(
            dossier.protocol_state_of("v4_echo_greeting"),
            Some(ProtocolState::Resolved(ProtocolVerdict::Compliant))
        );
    }

    #[test]
    fn a_resolved_protocol_verdict_survives_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let condition = ProtocolConditionDefinition {
            id: "v4_echo_greeting",
            chapter: "05",
            citation: "AfirmaWebSocketServerV4.java:81-83",
            statement: "La petición de eco recibe OK",
        };
        let mut dossier =
            Dossier::open(&path, "autofirma", &[], &[condition], Some(coordinates)).unwrap();
        dossier
            .resolve_protocol(
                "v4_echo_greeting",
                ProtocolVerdict::Discrepant,
                Some("error".to_owned()),
            )
            .unwrap();

        let reopened = Dossier::open(&path, "autofirma", &[], &[condition], None).unwrap();

        assert_eq!(
            reopened.protocol_state_of("v4_echo_greeting"),
            Some(ProtocolState::Resolved(ProtocolVerdict::Discrepant))
        );
        let (_, record) = reopened
            .protocol_conditions()
            .find(|(id, _)| *id == "v4_echo_greeting")
            .unwrap();
        assert_eq!(record.observation.as_deref(), Some("error"));
        assert_eq!(record.chapter, "05");
        assert_eq!(record.citation, "AfirmaWebSocketServerV4.java:81-83");
    }

    #[test]
    fn a_protocol_condition_not_yet_run_stays_pending() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let coordinates = HeaderCoordinates {
            os: "linux".to_owned(),
            os_version: "6.0".to_owned(),
            subject_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
            store: "softhsm2".to_owned(),
        };
        let condition = ProtocolConditionDefinition {
            id: "v4_echo_greeting",
            chapter: "05",
            citation: "AfirmaWebSocketServerV4.java:81-83",
            statement: "La petición de eco recibe OK",
        };
        let dossier =
            Dossier::open(&path, "autofirma", &[], &[condition], Some(coordinates)).unwrap();

        assert_eq!(
            dossier.protocol_state_of("v4_echo_greeting"),
            Some(ProtocolState::Pending)
        );
    }
}
