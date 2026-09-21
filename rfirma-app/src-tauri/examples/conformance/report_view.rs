//! Lo que se ve de un informe —sus conjuntos en el orden del catálogo, con sus recuentos y el
//! resultado de cada comprobación—, igual lo esté corriendo la sesión o no; no sabe de la sesión.

use serde::Serialize;

use crate::baseline::{verdict_name, PENDING_NAME};
use crate::catalogue::Check;
use crate::dossier::{CheckRecord, CheckState, Dossier, Header, Verdict};

#[derive(Debug, Serialize)]
pub(crate) struct ReportView<'a> {
    subject: &'a str,
    profile: &'static str,
    header: &'a Header,
    summary: Summary,
    suites: Vec<SuiteView<'a>>,
}

#[derive(Debug, Serialize)]
struct SuiteView<'a> {
    name: &'a str,
    summary: Summary,
    checks: Vec<CheckView<'a>>,
}

#[derive(Debug, Serialize)]
struct CheckView<'a> {
    id: &'a str,
    chapter: &'a str,
    statement: &'a str,
    citation: &'a str,
    warning: Option<&'a str>,
    question: Option<&'a str>,
    state: &'static str,
    observation: Option<&'a str>,
    date: Option<&'a str>,
    duration_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) struct Summary {
    pub total: usize,
    pub compliant: usize,
    pub noncompliant: usize,
    pub not_observable: usize,
    pub pending: usize,
}

impl Summary {
    fn count(&mut self, state: Option<CheckState>) {
        self.total += 1;
        match state {
            None | Some(CheckState::Pending) => self.pending += 1,
            Some(CheckState::Resolved(Verdict::Compliant)) => self.compliant += 1,
            Some(CheckState::Resolved(Verdict::Noncompliant)) => self.noncompliant += 1,
            Some(CheckState::Resolved(Verdict::NotObservable)) => self.not_observable += 1,
        }
    }
}

/// El nombre del resultado de una comprobación, que es PENDIENTE si el informe no la tiene.
pub(crate) fn result_name(state: Option<CheckState>) -> &'static str {
    match state {
        Some(CheckState::Resolved(verdict)) => verdict_name(verdict),
        _ => PENDING_NAME,
    }
}

pub(crate) fn report_view<'a>(dossier: &'a Dossier, catalogue: &'a [Check]) -> ReportView<'a> {
    let mut summary = Summary::default();
    let mut suites: Vec<SuiteView> = Vec::new();
    for check in catalogue {
        let record = dossier.record_of(&check.id);
        let state = record.map(|record| record.state);
        summary.count(state);
        let view = check_view(check, record);
        match suites.last_mut() {
            Some(suite) if suite.name == check.suite => {
                suite.summary.count(state);
                suite.checks.push(view);
            }
            _ => {
                let mut suite_summary = Summary::default();
                suite_summary.count(state);
                suites.push(SuiteView {
                    name: &check.suite,
                    summary: suite_summary,
                    checks: vec![view],
                });
            }
        }
    }
    ReportView {
        subject: dossier.subject(),
        profile: dossier.profile().name(),
        header: dossier.header(),
        summary,
        suites,
    }
}

fn check_view<'a>(check: &'a Check, record: Option<&'a CheckRecord>) -> CheckView<'a> {
    CheckView {
        id: &check.id,
        chapter: &check.chapter,
        statement: &check.statement,
        citation: &check.citation,
        warning: check.warning.as_deref(),
        question: check.question.as_deref(),
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
    use crate::baseline::Profile;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::HeaderCoordinates;

    const THREE_CHECKS: &str = r#"
[[check]]
id = "a_greeting"
suite = "saludo"
chapter = "14"
citation = "A.java:1"
statement = "Saluda."
drive = { mode = "v4", script = "protocol-v4" }

[[check]]
id = "a_signature"
suite = "operaciones"
chapter = "16"
citation = "B.java:2"
statement = "Firma."
drive = { mode = "v4", script = "sign" }
question = "¿se pidió el PIN? [s/n]"

[[check]]
id = "a_save"
suite = "operaciones"
chapter = "16"
citation = "C.java:3"
statement = "Guarda."
drive = { mode = "v4", script = "save" }
"#;

    fn a_report_of(
        profile: Profile,
        binary: &str,
        catalogue: &[Check],
        save: Verdict,
    ) -> (tempfile::TempDir, Dossier) {
        let dir = tempfile::tempdir().unwrap();
        let mut dossier = Dossier::create(
            &dir.path().join("dossier.json"),
            binary,
            profile,
            catalogue,
            HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                subject_version: "1.9.2".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2:/m.so".to_owned(),
            },
        )
        .unwrap();
        dossier
            .resolve(
                "a_greeting",
                Verdict::Compliant,
                None,
                Duration::from_millis(900),
            )
            .unwrap();
        dossier
            .resolve(
                "a_save",
                save,
                Some("guardó".to_owned()),
                Duration::from_millis(1_500),
            )
            .unwrap();
        (dir, dossier)
    }

    fn the_json_of(view: &ReportView) -> Value {
        serde_json::to_value(view).unwrap()
    }

    fn the_check<'v>(json: &'v Value, id: &str) -> &'v Value {
        json["suites"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|suite| suite["checks"].as_array().unwrap())
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
    fn the_suites_follow_the_catalogue_order_each_with_its_counts() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report_of(
            Profile::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Verdict::Noncompliant,
        );

        let json = the_json_of(&report_view(&dossier, &catalogue));

        let suites: Vec<(&str, &Value)> = json["suites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|suite| (suite["name"].as_str().unwrap(), &suite["summary"]))
            .collect();
        assert_eq!(
            suites,
            [
                (
                    "saludo",
                    &json!({"total": 1, "compliant": 1, "noncompliant": 0, "not_observable": 0, "pending": 0})
                ),
                (
                    "operaciones",
                    &json!({"total": 2, "compliant": 0, "noncompliant": 1, "not_observable": 0, "pending": 1})
                ),
            ]
        );
        assert_eq!(
            json["summary"],
            json!({"total": 3, "compliant": 1, "noncompliant": 1, "not_observable": 0, "pending": 1})
        );
    }

    #[test]
    fn a_resolved_check_carries_its_result_and_nothing_about_the_client() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report_of(
            Profile::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Verdict::Compliant,
        );

        let json = the_json_of(&report_view(&dossier, &catalogue));
        let save = the_check(&json, "a_save");

        assert_eq!(save["state"], "CONFORME");
        assert_eq!(save["observation"], "guardó");
        assert_eq!(save["duration_ms"], 1_500);
        assert_eq!(save.get("expectation"), None);
        assert_eq!(save.get("activity"), None);
        assert_eq!(the_check(&json, "a_signature")["state"], "PENDIENTE");
    }

    #[test]
    fn a_check_the_report_lacks_is_pending() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report_of(
            Profile::Rfirma,
            "/usr/bin/rfirma",
            &catalogue[..2],
            Verdict::Compliant,
        );

        let json = the_json_of(&report_view(&dossier, &catalogue));

        assert_eq!(the_check(&json, "a_save")["state"], "PENDIENTE");
        assert_eq!(json["summary"]["pending"], 2);
    }

    #[test]
    fn reports_of_different_clients_look_the_same() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_a, autofirma) = a_report_of(
            Profile::Autofirma,
            "/usr/bin/autofirma",
            &catalogue,
            Verdict::Noncompliant,
        );
        let (_b, rfirma) = a_report_of(
            Profile::Rfirma,
            "/usr/bin/rfirma",
            &catalogue,
            Verdict::Compliant,
        );

        let a = the_json_of(&report_view(&autofirma, &catalogue));
        let b = the_json_of(&report_view(&rfirma, &catalogue));

        assert_eq!(the_shape_of(&a), the_shape_of(&b));
        assert_eq!(a["profile"], "autofirma");
        assert_eq!(b["profile"], "rfirma");
        assert_ne!(
            the_check(&a, "a_save")["state"],
            the_check(&b, "a_save")["state"]
        );
    }
}
