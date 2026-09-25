//! Lo que se ve de un informe —sus conjuntos en el orden del catálogo, con sus recuentos y el
//! resultado de cada comprobación—, igual lo esté corriendo la sesión o no; no sabe de la sesión.

use serde::Serialize;
use ts_rs::TS;

use crate::catalogue::{Assistance, Check};
use crate::client::{ClientKind, Store};
use crate::known_bug::KnownBug;
use crate::outcome::{result_name, ResultName};
use crate::outcome::{CheckState, Outcome};
use crate::report::{CheckRecord, Header, Report};

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct ReportView<'a> {
    client: &'a str,
    #[ts(as = "ClientKind")]
    kind: &'static str,
    header: &'a Header,
    summary: Summary,
    sets: Vec<SetView<'a>>,
    orphans: Vec<OrphanView<'a>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct OrphanView<'a> {
    id: &'a str,
    #[ts(as = "ResultName")]
    state: &'static str,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct SetView<'a> {
    name: &'a str,
    summary: Summary,
    checks: Vec<CheckView<'a>>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct CheckView<'a> {
    id: &'a str,
    chapter: &'a str,
    statement: &'a str,
    citation: &'a str,
    warning: Option<&'a str>,
    assistance: Option<Assistance>,
    store: Store,
    bug: Option<&'a KnownBug>,
    deprecated: bool,
    #[ts(as = "ResultName")]
    state: &'static str,
    observation: Option<&'a str>,
    date: Option<&'a str>,
    #[ts(type = "number | null")]
    duration_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub(crate) struct Summary {
    pub total: usize,
    pub compliant: usize,
    /// Sin los de un formato deprecado, que no son un fallo del cliente y van en `deprecated`.
    pub noncompliant: usize,
    pub deprecated: usize,
    pub not_observable: usize,
    pub pending: usize,
}

impl Summary {
    fn count(&mut self, check: &Check, state: Option<CheckState>) {
        self.total += 1;
        match state {
            None | Some(CheckState::Pending) => self.pending += 1,
            Some(CheckState::Resolved(Outcome::Compliant)) => self.compliant += 1,
            Some(CheckState::Resolved(Outcome::Noncompliant)) if check.deprecated => {
                self.deprecated += 1;
            }
            Some(CheckState::Resolved(Outcome::Noncompliant)) => self.noncompliant += 1,
            Some(CheckState::Resolved(Outcome::NotObservable)) => self.not_observable += 1,
        }
    }
}

pub(crate) fn report_view<'a>(report: &'a Report, catalogue: &'a [Check]) -> ReportView<'a> {
    let mut summary = Summary::default();
    let mut sets: Vec<SetView> = Vec::new();
    for check in catalogue {
        let record = report.record_of(&check.id);
        let state = record.map(|record| record.state);
        summary.count(check, state);
        let view = check_view(check, record);
        match sets.last_mut() {
            Some(set) if set.name == check.requirement.set => {
                set.summary.count(check, state);
                set.checks.push(view);
            }
            _ => {
                let mut set_summary = Summary::default();
                set_summary.count(check, state);
                sets.push(SetView {
                    name: &check.requirement.set,
                    summary: set_summary,
                    checks: vec![view],
                });
            }
        }
    }
    ReportView {
        client: report.client(),
        kind: report.kind().name(),
        header: report.header(),
        summary,
        sets,
        orphans: report
            .orphans()
            .map(|(id, state)| OrphanView {
                id,
                state: result_name(Some(state)),
            })
            .collect(),
    }
}

fn check_view<'a>(check: &'a Check, record: Option<&'a CheckRecord>) -> CheckView<'a> {
    CheckView {
        id: &check.id,
        chapter: &check.requirement.chapter,
        statement: &check.requirement.statement,
        citation: &check.requirement.citation,
        warning: check.instruction(),
        assistance: check.trial().map(|trial| trial.act.assistance()),
        store: check.store(),
        bug: check.bug,
        deprecated: check.deprecated,
        state: result_name(record.map(|record| record.state)),
        observation: record.and_then(|record| record.observation.as_deref()),
        date: record.and_then(|record| record.date.as_deref()),
        duration_ms: record.and_then(|record| record.duration_ms),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{json, Value};

    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::client::ClientKind;
    use crate::report::HeaderCoordinates;

    const THREE_CHECKS: &str = r#"
[[check]]
id = "a_greeting"
set = "saludo"
chapter = "14"
citation = "A.java:1"
statement = "Saluda."

[check.drive]
mode = "v4"
script = "protocol-v4"
expects.completes = {}

[[check]]
id = "a_signature"
set = "operaciones"
chapter = "16"
citation = "B.java:2"
statement = "Firma."

[check.drive]
mode = "v4"
script = "sign"
expects.completes = {}

[[check]]
id = "a_save"
set = "operaciones"
chapter = "16"
citation = "C.java:3"
statement = "Guarda."
bug = "BUG-18"

[check.drive]
mode = "v4"
script = "save"
expects.completes = {}
"#;

    fn a_report_of(
        kind: ClientKind,
        binary: &str,
        catalogue: &[Check],
        save: Outcome,
    ) -> (tempfile::TempDir, Report) {
        let dir = tempfile::tempdir().unwrap();
        let mut report = Report::create(
            &dir.path().join("dossier.json"),
            binary,
            kind,
            catalogue,
            HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.9.2".to_owned(),
            },
        )
        .unwrap();
        report
            .resolve(
                "a_greeting",
                Outcome::Compliant,
                None,
                Duration::from_millis(900),
            )
            .unwrap();
        report
            .resolve(
                "a_save",
                save,
                Some("guardó".to_owned()),
                Duration::from_millis(1_500),
            )
            .unwrap();
        (dir, report)
    }

    fn the_json_of(view: &ReportView) -> Value {
        serde_json::to_value(view).unwrap()
    }

    fn the_check<'v>(json: &'v Value, id: &str) -> &'v Value {
        json["sets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|set| set["checks"].as_array().unwrap())
            .find(|check| check["id"] == id)
            .unwrap()
    }

    fn the_shape_of(json: &Value) -> Value {
        match json {
            Value::Object(fields) => fields
                .iter()
                .map(|(key, value)| (key.clone(), the_shape_of(value)))
                .collect::<serde_json::Map<_, _>>()
                .into(),
            Value::Array(items) => items.iter().map(the_shape_of).collect(),
            _ => Value::Null,
        }
    }

    #[test]
    fn the_sets_follow_the_catalogue_order_each_with_its_counts() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, report) = a_report_of(
            ClientKind::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Outcome::Noncompliant,
        );

        let json = the_json_of(&report_view(&report, &catalogue));

        let sets: Vec<(&str, &Value)> = json["sets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|set| (set["name"].as_str().unwrap(), &set["summary"]))
            .collect();
        assert_eq!(
            sets,
            [
                (
                    "saludo",
                    &json!({"total": 1, "compliant": 1, "noncompliant": 0, "deprecated": 0, "not_observable": 0, "pending": 0})
                ),
                (
                    "operaciones",
                    &json!({"total": 2, "compliant": 0, "noncompliant": 1, "deprecated": 0, "not_observable": 0, "pending": 1})
                ),
            ]
        );
        assert_eq!(
            json["summary"],
            json!({"total": 3, "compliant": 1, "noncompliant": 1, "deprecated": 0, "not_observable": 0, "pending": 1})
        );
    }

    #[test]
    fn a_noncompliance_in_a_deprecated_format_is_counted_apart_from_the_failures() {
        let deprecated = THREE_CHECKS.replace("bug = \"BUG-18\"", "deprecated = true");
        let catalogue = the_catalogue_in(&deprecated).unwrap();
        let (_dir, report) = a_report_of(
            ClientKind::Rfirma,
            "/usr/bin/rfirma",
            &catalogue,
            Outcome::Noncompliant,
        );

        let json = the_json_of(&report_view(&report, &catalogue));

        assert_eq!(the_check(&json, "a_save")["deprecated"], true);
        assert_eq!(the_check(&json, "a_greeting")["deprecated"], false);
        assert_eq!(json["summary"]["noncompliant"], 0);
        assert_eq!(json["summary"]["deprecated"], 1);
    }

    #[test]
    fn a_check_the_catalogue_no_longer_has_is_an_orphan_and_counts_nowhere() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (dir, _) = a_report_of(
            ClientKind::Rfirma,
            "/usr/bin/rfirma",
            &catalogue,
            Outcome::Noncompliant,
        );
        let without_the_save = &THREE_CHECKS[..THREE_CHECKS.rfind("[[check]]").unwrap()];
        let current = the_catalogue_in(without_the_save).unwrap();

        let report = Report::open(&dir.path().join("dossier.json"), &current).unwrap();
        let json = the_json_of(&report_view(&report, &current));

        assert_eq!(json["summary"]["total"], 2);
        assert_eq!(json["summary"]["noncompliant"], 0);
        assert_eq!(
            json["orphans"],
            json!([{ "id": "a_save", "state": "NO CONFORME" }])
        );
    }

    #[test]
    fn a_resolved_check_carries_its_result_and_nothing_about_the_client() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, report) = a_report_of(
            ClientKind::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Outcome::Compliant,
        );

        let json = the_json_of(&report_view(&report, &catalogue));
        let save = the_check(&json, "a_save");

        assert_eq!(save["state"], "CONFORME");
        assert_eq!(save["observation"], "guardó");
        assert_eq!(save["duration_ms"], 1_500);
        assert_eq!(save.get("expectation"), None);
        assert_eq!(save.get("activity"), None);
        assert_eq!(the_check(&json, "a_signature")["state"], "PENDIENTE");
    }

    #[test]
    fn a_check_carries_the_known_bug_of_the_original_with_its_state_in_master() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, report) = a_report_of(
            ClientKind::Rfirma,
            "/usr/bin/rfirma",
            &catalogue,
            Outcome::Compliant,
        );

        let json = the_json_of(&report_view(&report, &catalogue));

        assert_eq!(the_check(&json, "a_save")["bug"]["id"], "BUG-18");
        assert_eq!(the_check(&json, "a_save")["bug"]["master"], "fixed");
        assert_eq!(the_check(&json, "a_greeting")["bug"], Value::Null);
    }

    #[test]
    fn a_check_the_report_lacks_is_pending() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, report) = a_report_of(
            ClientKind::Rfirma,
            "/usr/bin/rfirma",
            &catalogue[..2],
            Outcome::Compliant,
        );

        let json = the_json_of(&report_view(&report, &catalogue));

        assert_eq!(the_check(&json, "a_save")["state"], "PENDIENTE");
        assert_eq!(json["summary"]["pending"], 2);
    }

    #[test]
    fn reports_of_different_clients_look_the_same() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_a, autofirma) = a_report_of(
            ClientKind::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Outcome::Noncompliant,
        );
        let (_b, rfirma) = a_report_of(
            ClientKind::Rfirma,
            "/usr/bin/rfirma",
            &catalogue,
            Outcome::Compliant,
        );

        let a = the_json_of(&report_view(&autofirma, &catalogue));
        let b = the_json_of(&report_view(&rfirma, &catalogue));

        assert_eq!(the_shape_of(&a), the_shape_of(&b));
        assert_eq!(a["kind"], "autofirma");
        assert_eq!(b["kind"], "rfirma");
        assert_ne!(
            the_check(&a, "a_save")["state"],
            the_check(&b, "a_save")["state"]
        );
    }
}
