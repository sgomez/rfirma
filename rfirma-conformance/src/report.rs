//! El informe de la suite de conformidad: qué comprobaciones se han corrido, con qué cliente y
//! desde cuándo.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::catalogue::Check;
use crate::client::{ClientKind, Origin};
use crate::errand::{ErrandKey, ObservedErrand};
use crate::outcome::{CheckState, Outcome};
use crate::selection::resolve;

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
    pub date: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub notes: String,
    /// La selección que mide, tal como se nombró; `None` es el catálogo entero.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Lo que se puede corregir de la cabecera de un informe ya creado; la fecha no.
#[derive(Debug, Clone, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct HeaderCorrection {
    pub title: String,
    pub notes: String,
    pub client_version: String,
    pub os: String,
    pub os_version: String,
    pub scope: Option<String>,
}

/// Las coordenadas que da quien crea el informe, sin la fecha: esa la pone el informe.
pub struct HeaderCoordinates {
    pub os: String,
    pub os_version: String,
    pub client_version: String,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Contents {
    client: String,
    kind: ClientKind,
    #[serde(default)]
    origin: Option<Origin>,
    header: Header,
    checks: BTreeMap<String, CheckRecord>,
    #[serde(default)]
    errands: BTreeMap<ErrandKey, ObservedErrand>,
}

/// El nombre del fichero del informe dentro de su directorio.
pub const THE_REPORT_FILE: &str = "dossier.json";

/// El informe: un JSON en la ruta que se le indique, que continúa un cliente de su clase y su versión.
#[derive(Debug)]
pub struct Report {
    path: PathBuf,
    contents: Contents,
    orphans: BTreeMap<String, CheckRecord>,
    /// Lo que resuelve el alcance contra el catálogo con que se abrió; `None` es todo.
    in_scope: Option<BTreeSet<String>>,
}

impl Report {
    /// Crea un informe nuevo en `path`, con `catalogue` entero pendiente.
    pub fn create(
        path: &Path,
        client: &str,
        kind: ClientKind,
        origin: Origin,
        catalogue: &[Check],
        coordinates: HeaderCoordinates,
    ) -> Result<Self, String> {
        let mut report = Self {
            path: path.to_owned(),
            contents: Contents {
                client: client.to_owned(),
                kind,
                origin: Some(origin),
                header: Header {
                    os: coordinates.os,
                    os_version: coordinates.os_version,
                    client_version: coordinates.client_version,
                    date: today(),
                    title: String::new(),
                    notes: String::new(),
                    scope: coordinates.scope,
                },
                checks: BTreeMap::new(),
                errands: BTreeMap::new(),
            },
            orphans: BTreeMap::new(),
            in_scope: None,
        };
        report.cover(catalogue);
        report.save()?;
        Ok(report)
    }

    /// Abre un informe ya escrito, con cualquier cliente, da por pendiente lo que `catalogue`
    /// tenga y él no, y aparta como huérfano lo que él tenga y `catalogue` no; no escribe nada.
    pub fn open(path: &Path, catalogue: &[Check]) -> Result<Self, String> {
        let mut report = Self::read(path)?;
        report.set_apart_the_orphans(catalogue);
        report.cover(catalogue);
        Ok(report)
    }

    fn set_apart_the_orphans(&mut self, catalogue: &[Check]) {
        let current: BTreeSet<&str> = catalogue.iter().map(|check| check.id.as_str()).collect();
        let (kept, orphans) = std::mem::take(&mut self.contents.checks)
            .into_iter()
            .partition(|(id, _)| current.contains(id.as_str()));
        self.contents.checks = kept;
        self.orphans = orphans;
    }

    /// Lo que el informe guardaba de comprobaciones que el catálogo ya no tiene, con su estado.
    pub fn orphans(&self) -> impl Iterator<Item = (&str, CheckState)> {
        self.orphans
            .iter()
            .map(|(id, record)| (id.as_str(), record.state))
    }

    /// Si `id` es de lo que mide el informe; lo de fuera ni cuenta ni se encola.
    pub fn covers(&self, id: &str) -> bool {
        self.in_scope
            .as_ref()
            .is_none_or(|in_scope| in_scope.contains(id))
    }

    /// Resuelve el alcance contra `catalogue` y da por pendiente lo que mide y aún no tiene.
    fn cover(&mut self, catalogue: &[Check]) {
        self.in_scope = self
            .contents
            .header
            .scope
            .as_deref()
            .map(|scope| resolve(scope, catalogue, None).ids.into_iter().collect());
        let measured: Vec<String> = catalogue
            .iter()
            .filter(|check| self.covers(&check.id))
            .map(|check| check.id.clone())
            .collect();
        for id in measured {
            self.contents
                .checks
                .entry(id)
                .or_insert_with(CheckRecord::pending);
        }
    }

    /// Por qué un cliente de clase `kind` que dice ser la versión `version` no puede añadir
    /// resultados a este informe; `None` si es de su clase y de su versión, sea cual sea su ruta.
    pub fn refuses_to_continue_with(
        &self,
        kind: ClientKind,
        version: Option<&str>,
    ) -> Option<String> {
        if self.contents.kind != kind {
            return Some(format!(
                "el informe se creó para {} y el cliente activo es {}",
                self.contents.kind.name(),
                kind.name()
            ));
        }
        let made_with = &self.contents.header.client_version;
        version
            .filter(|version| version != made_with)
            .map(|version| {
                format!(
                    "el informe se creó con la versión {made_with} y el cliente activo es la \
                     {version}"
                )
            })
    }

    pub fn client(&self) -> &str {
        &self.contents.client
    }

    pub fn kind(&self) -> ClientKind {
        self.contents.kind
    }

    /// De dónde salía el binario con que se creó; `None` en un informe de antes de guardarlo.
    pub fn origin(&self) -> Option<Origin> {
        self.contents.origin
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
            orphans: BTreeMap::new(),
            in_scope: None,
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

    /// Deja `id` pendiente y olvida el trámite de `key`, para que la próxima vez se lance de nuevo.
    pub fn forget(&mut self, id: &str, key: Option<&ErrandKey>) -> Result<(), String> {
        if let Some(record) = self.contents.checks.get_mut(id) {
            *record = CheckRecord::pending();
        }
        if let Some(key) = key {
            self.contents.errands.remove(key);
        }
        self.save()
    }

    /// Corrige la cabecera, con el alcance resuelto contra `catalogue`, y la deja escrita antes de
    /// devolver el control; lo que queda fuera se conserva sin contar.
    pub fn correct_header(
        &mut self,
        correction: HeaderCorrection,
        catalogue: &[Check],
    ) -> Result<(), String> {
        let header = &mut self.contents.header;
        header.title = correction.title;
        header.notes = correction.notes;
        header.client_version = correction.client_version;
        header.os = correction.os;
        header.os_version = correction.os_version;
        header.scope = correction.scope;
        self.cover(catalogue);
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
                     \n[check.drive]\nmode = \"v4\"\nscript = \"selectcert\"\n\
                     expects.code = \"SAF_03\"\n\n"
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
            scope: None,
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
            Origin::Installed,
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
            Origin::Installed,
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
            Origin::Installed,
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
            Origin::Installed,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        let seen = Report::open(&path, &catalogue).unwrap();
        let complaint = seen
            .refuses_to_continue_with(ClientKind::Rfirma, Some("1.9.2"))
            .unwrap();

        assert!(complaint.contains("para autofirma y el cliente activo es rfirma"));
    }

    #[test]
    fn a_report_continues_with_a_binary_elsewhere_of_its_class_and_version() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let report = Report::create(
            &path,
            "/usr/bin/rfirma",
            ClientKind::Rfirma,
            Origin::Installed,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(
            report.refuses_to_continue_with(ClientKind::Rfirma, Some("1.9.2")),
            None
        );
        let complaint = report
            .refuses_to_continue_with(ClientKind::Rfirma, Some("2.0.0"))
            .unwrap();
        assert!(
            complaint.contains("con la versión 1.9.2 y el cliente activo es la 2.0.0"),
            "{complaint}"
        );
    }

    #[test]
    fn a_client_that_does_not_say_its_version_continues_a_report_of_its_class() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let report = Report::create(
            &path,
            "/usr/bin/autofirma",
            ClientKind::Autofirma,
            Origin::Installed,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(
            report.refuses_to_continue_with(ClientKind::Autofirma, None),
            None
        );
    }

    #[test]
    fn a_check_the_catalogue_no_longer_has_is_set_apart_and_pruned_on_the_next_save() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            Origin::Installed,
            &a_catalogue_of(&["v4_echo_greeting", "a_renamed_one"]),
            some_coordinates(),
        )
        .unwrap();

        let mut report = Report::open(&path, &a_catalogue_of(&["v4_echo_greeting"])).unwrap();

        assert_eq!(report.state_of("a_renamed_one"), None);
        assert_eq!(
            report.orphans().collect::<Vec<_>>(),
            [("a_renamed_one", CheckState::Pending)]
        );
        report
            .resolve("v4_echo_greeting", Outcome::Compliant, None, Duration::ZERO)
            .unwrap();
        assert!(!std::fs::read_to_string(&path)
            .unwrap()
            .contains("a_renamed_one"));
    }

    #[test]
    fn opening_a_report_to_see_it_writes_nothing() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            Origin::Installed,
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
    fn a_report_whose_header_still_carries_a_transport_opens() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            Origin::Installed,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();
        let mut written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        written["header"]["transport"] = "websocket".into();
        std::fs::write(&path, written.to_string()).unwrap();

        let seen = Report::read(&path).unwrap();

        assert_eq!(seen.header().client_version, "1.9.2");
    }

    #[test]
    fn the_client_kind_of_a_report_survives_a_reopen() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = a_catalogue_of(&["v4_echo_greeting"]);
        Report::create(
            &path,
            "un-binario",
            ClientKind::Rfirma,
            Origin::Installed,
            &catalogue,
            some_coordinates(),
        )
        .unwrap();

        assert_eq!(Report::read(&path).unwrap().kind(), ClientKind::Rfirma);
    }

    #[test]
    fn a_new_report_keeps_the_identity_of_its_client() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "/src/rfirma/target/release/rfirma",
            ClientKind::Rfirma,
            Origin::LocalBuild,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();

        let seen = Report::read(&path).unwrap();

        assert_eq!(seen.kind(), ClientKind::Rfirma);
        assert_eq!(seen.origin(), Some(Origin::LocalBuild));
        assert_eq!(seen.header().client_version, "1.9.2");
    }

    #[test]
    fn a_report_written_before_the_origin_is_still_read() {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            "/usr/bin/rfirma",
            ClientKind::Rfirma,
            Origin::Installed,
            &a_catalogue_of(&["v4_echo_greeting"]),
            some_coordinates(),
        )
        .unwrap();
        let mut written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        written.as_object_mut().unwrap().remove("origin");
        std::fs::write(&path, written.to_string()).unwrap();

        let seen = Report::read(&path).unwrap();

        assert_eq!(seen.origin(), None);
        assert_eq!(seen.client(), "/usr/bin/rfirma");
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
            Origin::Installed,
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
