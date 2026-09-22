//! El informe de la suite de conformidad: qué comprobaciones se han corrido, con qué cliente y
//! desde cuándo.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::catalogue::Check;
use crate::client::ClientKind;
use crate::errand::{ErrandKey, ObservedErrand};
use crate::outcome::{CheckState, Outcome};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRecord {
    pub state: CheckState,
    pub date: Option<String>,
    #[serde(default)]
    pub observation: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl CheckRecord {
    fn pending() -> Self {
        Self {
            state: CheckState::Pending,
            date: None,
            observation: None,
            duration_ms: None,
        }
    }
}

/// Las coordenadas de un informe, tomadas una sola vez al crearlo.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Header {
    pub os: String,
    pub os_version: String,
    pub client_version: String,
    pub transport: String,
    pub date: String,
}

/// Las coordenadas que da quien crea el informe, sin la fecha: esa la pone el informe.
pub struct HeaderCoordinates {
    pub os: String,
    pub os_version: String,
    pub client_version: String,
    pub transport: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Contents {
    client: String,
    kind: ClientKind,
    header: Header,
    checks: BTreeMap<String, CheckRecord>,
    #[serde(default)]
    errands: BTreeMap<ErrandKey, ObservedErrand>,
}

/// El nombre del fichero del informe dentro de su directorio.
pub const THE_REPORT_FILE: &str = "dossier.json";

/// El informe: un JSON en la ruta que se le indique, que solo continúa el cliente
/// con que se creó.
#[derive(Debug)]
pub struct Report {
    path: PathBuf,
    contents: Contents,
}

impl Report {
    /// Crea un informe nuevo en `path`, con `catalogue` entero pendiente.
    pub fn create(
        path: &Path,
        client: &str,
        kind: ClientKind,
        catalogue: &[Check],
        coordinates: HeaderCoordinates,
    ) -> Result<Self, String> {
        let mut report = Self {
            path: path.to_owned(),
            contents: Contents {
                client: client.to_owned(),
                kind,
                header: Header {
                    os: coordinates.os,
                    os_version: coordinates.os_version,
                    client_version: coordinates.client_version,
                    transport: coordinates.transport,
                    date: today(),
                },
                checks: BTreeMap::new(),
                errands: BTreeMap::new(),
            },
        };
        report.cover(catalogue);
        report.save()?;
        Ok(report)
    }

    /// Abre un informe ya escrito, con cualquier cliente, y da por pendiente lo que `catalogue`
    /// tenga y él no; no escribe nada.
    pub fn open(path: &Path, catalogue: &[Check]) -> Result<Self, String> {
        let mut report = Self::read(path)?;
        report.cover(catalogue);
        Ok(report)
    }

    fn cover(&mut self, catalogue: &[Check]) {
        for check in catalogue {
            self.contents
                .checks
                .entry(check.id.clone())
                .or_insert_with(CheckRecord::pending);
        }
    }

    /// Por qué el cliente `client` de clase `kind` no puede añadir resultados a este
    /// informe; `None` si es el mismo con que se creó.
    pub fn refuses_to_continue_with(&self, client: &str, kind: ClientKind) -> Option<String> {
        let mut mismatches = Vec::new();
        if self.contents.kind != kind {
            mismatches.push(format!(
                "se creó para {} y el cliente activo es {}",
                self.contents.kind.name(),
                kind.name()
            ));
        }
        if self.contents.client != client {
            mismatches.push(format!(
                "se creó con {} y el cliente activo es {client}",
                self.contents.client
            ));
        }
        (!mismatches.is_empty()).then(|| {
            format!(
                "el informe {}: se puede ver, pero no continuar",
                mismatches.join(", y ")
            )
        })
    }

    pub fn client(&self) -> &str {
        &self.contents.client
    }

    pub fn kind(&self) -> ClientKind {
        self.contents.kind
    }

    /// Lee un informe ya escrito, sin catálogo ni cliente con el que cuadrarlo: lo que necesita
    /// quien compara dos informes.
    pub fn read(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path)
            .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
        let contents = serde_json::from_str(&raw)
            .map_err(|error| format!("{} no es un informe válido: {error}", path.display()))?;
        Ok(Self {
            path: path.to_owned(),
            contents,
        })
    }

    pub fn header(&self) -> &Header {
        &self.contents.header
    }

    #[cfg(test)]
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

    /// Marca `id` con `outcome` hoy, junto a `observation` si la hubo y lo que tardó, y lo deja
    /// escrito antes de devolver el control.
    pub fn resolve(
        &mut self,
        id: &str,
        outcome: Outcome,
        observation: Option<String>,
        duration: Duration,
    ) -> Result<(), String> {
        if let Some(record) = self.contents.checks.get_mut(id) {
            record.state = CheckState::Resolved(outcome);
            record.date = Some(today());
            record.observation = observation;
            record.duration_ms = Some(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX));
        }
        self.save()
    }

    /// El trámite ya observado con esa clave, que cualquier comprobación que la comparta juzga sin
    /// relanzar el cliente.
    pub fn observed(&self, key: &ErrandKey) -> Option<&ObservedErrand> {
        self.contents.errands.get(key)
    }

    /// Guarda el trámite observado con su clave y lo deja escrito antes de devolver el control.
    pub fn observe(&mut self, key: ErrandKey, observed: ObservedErrand) -> Result<(), String> {
        self.contents.errands.insert(key, observed);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.contents)
            .map_err(|error| format!("el informe no se pudo serializar: {error}"))?;
        fs::write(&self.path, json)
            .map_err(|error| format!("{} no se pudo escribir: {error}", self.path.display()))
    }
}

/// La fecha de hoy, `AAAA-MM-DD`: la misma que usa el informe para fechar un resultado.
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
                    "[[check]]\nid = \"{id}\"\nset = \"errores\"\nchapter = \"15\"\n\
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
            client_version: "1.9.2".to_owned(),
            transport: "websocket".to_owned(),
        }
    }

    #[test]
    fn resolves_a_check_with_the_single_outcome() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let mut report = Report::create(
            &path,
            "autofirma",
            ClientKind::Autofirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(
            report.state_of("v4_echo_greeting"),
            Some(CheckState::Pending)
        );

        report
            .resolve(
                "v4_echo_greeting",
                Outcome::Noncompliant,
                None,
                Duration::ZERO,
            )
            .unwrap();

        assert_eq!(
            report.state_of("v4_echo_greeting"),
            Some(CheckState::Resolved(Outcome::Noncompliant))
        );
    }

    #[test]
    fn a_resolved_outcome_its_observation_and_its_duration_survive_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let mut report = Report::create(
            &path,
            "autofirma",
            ClientKind::Autofirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();
        report
            .resolve(
                "v4_echo_greeting",
                Outcome::Compliant,
                Some("OK".to_owned()),
                Duration::from_millis(1_250),
            )
            .unwrap();

        let reopened = Report::open(&path, &catalogue).unwrap();

        assert_eq!(
            reopened.state_of("v4_echo_greeting"),
            Some(CheckState::Resolved(Outcome::Compliant))
        );
        let (_, record) = reopened
            .checks()
            .find(|(id, _)| *id == "v4_echo_greeting")
            .unwrap();
        assert_eq!(record.observation.as_deref(), Some("OK"));
        assert_eq!(record.duration_ms, Some(1_250));
    }

    #[test]
    fn a_check_not_yet_run_stays_pending_instead_of_being_omitted() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting", "v3_echo_greeting"]);
        let report = Report::create(
            &path,
            "autofirma",
            ClientKind::Autofirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        let ids: Vec<&str> = report.checks().map(|(id, _)| id).collect();
        assert!(ids.contains(&"v3_echo_greeting"));
        assert_eq!(
            report.state_of("v3_echo_greeting"),
            Some(CheckState::Pending)
        );
    }

    #[test]
    fn a_report_of_another_client_opens_to_be_seen_but_refuses_to_continue() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Report::create(
            &path,
            "/usr/bin/autofirma",
            ClientKind::Autofirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        let seen = Report::open(&path, &catalogue).unwrap();
        let complaint = seen
            .refuses_to_continue_with("/usr/bin/rfirma", ClientKind::Rfirma)
            .unwrap();

        assert!(complaint.contains("para autofirma y el cliente activo es rfirma"));
        assert!(complaint.contains("/usr/bin/autofirma y el cliente activo es /usr/bin/rfirma"));
    }

    #[test]
    fn the_client_that_created_a_report_continues_it() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let report = Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(
            report.refuses_to_continue_with("un-binario", ClientKind::Rfirma),
            None
        );
    }

    #[test]
    fn opening_a_report_to_see_it_writes_nothing() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();
        let written = std::fs::read_to_string(&path).unwrap();

        let seen =
            Report::open(&path, &a_catalogue_of(&["v4_echo_greeting", "a_new_one"])).unwrap();

        assert_eq!(seen.state_of("a_new_one"), Some(CheckState::Pending));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), written);
    }

    #[test]
    fn the_client_kind_of_a_report_survives_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(Report::read(&path).unwrap().kind(), ClientKind::Rfirma);
    }

    #[test]
    fn an_observed_errand_survives_a_reopen_under_its_key() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        let key = ErrandKey::of(&catalogue[0]).unwrap();
        let observed = ObservedErrand {
            outcome: crate::errand::ErrandOutcome {
                launched: true,
                error_code: Some("SAF_03".to_owned()),
                ..Default::default()
            },
            transcribed_in: "v4_echo_greeting".to_owned(),
            duration_ms: 900,
        };
        let mut report = Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        report.observe(key.clone(), observed.clone()).unwrap();

        assert_eq!(
            Report::open(&path, &catalogue).unwrap().observed(&key),
            Some(&observed)
        );
    }
}
