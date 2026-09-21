//! El estado que la consola web recibe de un vistazo —qué corre, qué espera, veredictos y
//! resumen— calculado de un informe y un catálogo, sin tocar al sujeto.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::baseline::{
    contrast_of, the_expectation_of, the_surprises_of, verdict_name, BaselineTally,
};
use crate::catalogue::Check;
use crate::comparison::PENDING_NAME;
use crate::dossier::{CheckRecord, CheckState, Dossier, Header, Verdict};
use crate::subject::Subject;

/// Lo que está pasando ahora en la sesión, más allá de lo que ya quedó escrito en el informe.
#[derive(Default)]
pub(crate) struct Activity<'a> {
    pub(crate) running: &'a [String],
    pub(crate) running_for: Duration,
    pub(crate) queued: Vec<&'a str>,
    pub(crate) question: Option<(&'a str, &'a str, &'static str)>,
    pub(crate) reasons: Option<&'a BTreeMap<String, String>>,
    pub(crate) resolving_subject: bool,
}

/// Un informe del directorio de informes, tal y como se ofrece para elegirlo o compararlo.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReportEntry {
    pub(crate) name: String,
    pub(crate) profile: Option<&'static str>,
    pub(crate) subject: Option<String>,
    pub(crate) subject_version: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) complaint: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Snapshot<'a> {
    subject: Option<&'a Subject>,
    subject_complaints: &'a [String],
    resolving_subject: bool,
    report: Option<ReportView<'a>>,
    reports: &'a [ReportEntry],
    running: Option<RunningView<'a>>,
    queued: &'a [&'a str],
    question: Option<QuestionView<'a>>,
    suites: Vec<SuiteView<'a>>,
}

#[derive(Debug, Serialize)]
struct ReportView<'a> {
    name: &'a str,
    subject: &'a str,
    profile: &'static str,
    header: &'a Header,
    summary: Summary,
    baseline: BaselineView,
    green: bool,
    surprises: Vec<String>,
    resolved: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
struct BaselineView {
    matching: usize,
    surprises: usize,
    unmeasured: usize,
    pending: usize,
}

#[derive(Debug, Serialize)]
struct RunningView<'a> {
    ids: &'a [String],
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
struct QuestionView<'a> {
    check: &'a str,
    prompt: &'a str,
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct SuiteView<'a> {
    name: &'a str,
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
    expectation: Option<ExpectationView<'a>>,
    state: &'static str,
    observation: Option<&'a str>,
    date: Option<&'a str>,
    duration_ms: Option<u64>,
    contrast: Option<&'static str>,
    activity: Option<&'static str>,
    why_pending: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ExpectationView<'a> {
    verdict: &'static str,
    cause: Option<&'a str>,
    note: Option<&'a str>,
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
    pub(crate) fn of<'a>(records: impl Iterator<Item = &'a CheckRecord>) -> Self {
        let mut summary = Self::default();
        for record in records {
            summary.total += 1;
            match record.state {
                CheckState::Pending => summary.pending += 1,
                CheckState::Resolved(Verdict::Compliant) => summary.compliant += 1,
                CheckState::Resolved(Verdict::Noncompliant) => summary.noncompliant += 1,
                CheckState::Resolved(Verdict::NotObservable) => summary.not_observable += 1,
            }
        }
        summary
    }
}

/// El estado entero de la sesión para la página: `report` es el informe abierto con su nombre.
pub(crate) fn snapshot_of<'a>(
    catalogue: &'a [Check],
    subject: Option<&'a Subject>,
    subject_complaints: &'a [String],
    report: Option<(&'a str, &'a Dossier)>,
    reports: &'a [ReportEntry],
    activity: &'a Activity<'a>,
) -> Snapshot<'a> {
    let profile = report.map(|(_, dossier)| dossier.profile());
    let mut suites: Vec<SuiteView> = Vec::new();
    for check in catalogue {
        let record = report.and_then(|(_, dossier)| dossier.record_of(&check.id));
        let expectation = profile.and_then(|profile| check.expect.get(profile.name()));
        let view = CheckView {
            id: &check.id,
            chapter: &check.chapter,
            statement: &check.statement,
            citation: &check.citation,
            warning: check.warning.as_deref(),
            question: check.question.as_deref(),
            expectation: expectation.map(|expectation| ExpectationView {
                verdict: verdict_name(expectation.verdict),
                cause: expectation.cause.as_deref(),
                note: expectation.note.as_deref(),
            }),
            state: match record.map(|record| record.state) {
                Some(CheckState::Resolved(verdict)) => verdict_name(verdict),
                _ => PENDING_NAME,
            },
            observation: record.and_then(|record| record.observation.as_deref()),
            date: record.and_then(|record| record.date.as_deref()),
            duration_ms: record.and_then(|record| record.duration_ms),
            contrast: match (record.map(|record| record.state), expectation) {
                (Some(CheckState::Resolved(observed)), Some(expectation)) => {
                    Some(contrast_of(observed, expectation.verdict).label())
                }
                _ => None,
            },
            activity: the_activity_of(&check.id, activity),
            why_pending: activity
                .reasons
                .and_then(|reasons| reasons.get(&check.id))
                .map(String::as_str)
                .filter(|_| !matches!(record.map(|r| r.state), Some(CheckState::Resolved(_)))),
        };
        match suites.last_mut() {
            Some(suite) if suite.name == check.suite => suite.checks.push(view),
            _ => suites.push(SuiteView {
                name: &check.suite,
                checks: vec![view],
            }),
        }
    }
    Snapshot {
        subject,
        subject_complaints,
        resolving_subject: activity.resolving_subject,
        report: report.map(|(name, dossier)| the_report_view(catalogue, name, dossier)),
        reports,
        running: (!activity.running.is_empty()).then_some(RunningView {
            ids: activity.running,
            elapsed_ms: activity.running_for.as_millis(),
        }),
        queued: &activity.queued,
        question: activity.question.map(|(check, prompt, kind)| QuestionView {
            check,
            prompt,
            kind,
        }),
        suites,
    }
}

fn the_activity_of(id: &str, activity: &Activity) -> Option<&'static str> {
    if activity.question.is_some_and(|(check, _, _)| check == id) {
        Some("asking")
    } else if activity.running.iter().any(|running| running == id) {
        Some("running")
    } else if activity.queued.contains(&id) {
        Some("queued")
    } else {
        None
    }
}

fn the_report_view<'a>(catalogue: &[Check], name: &'a str, dossier: &'a Dossier) -> ReportView<'a> {
    let profile = dossier.profile();
    let shown: Vec<(&str, &CheckRecord)> = catalogue
        .iter()
        .filter_map(|check| {
            dossier
                .record_of(&check.id)
                .map(|record| (check.id.as_str(), record))
        })
        .collect();
    let summary = Summary::of(shown.iter().map(|(_, record)| *record));
    let tally = BaselineTally::of(
        shown
            .iter()
            .map(|(id, record)| (*record, the_expectation_of(catalogue, id, profile))),
    );
    let surprises = the_surprises_of(
        shown
            .iter()
            .map(|(id, record)| (*id, *record, the_expectation_of(catalogue, id, profile))),
    );
    ReportView {
        name,
        subject: dossier.subject(),
        profile: profile.name(),
        header: dossier.header(),
        summary,
        baseline: BaselineView {
            matching: tally.matching,
            surprises: tally.surprises,
            unmeasured: tally.unmeasured,
            pending: tally.pending,
        },
        green: tally.is_green(),
        surprises,
        resolved: summary.total - summary.pending,
        total: summary.total,
    }
}

#[cfg(test)]
mod tests {
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
expect.autofirma = { verdict = "conforme" }

[[check]]
id = "a_signature"
suite = "operaciones"
chapter = "16"
citation = "B.java:2"
statement = "Firma."
drive = { mode = "v4", script = "sign" }
question = "¿se pidió el PIN? [s/n]"
expect.autofirma = { verdict = "conforme" }

[[check]]
id = "a_save"
suite = "operaciones"
chapter = "16"
citation = "C.java:3"
statement = "Guarda."
drive = { mode = "v4", script = "save" }
expect.autofirma = { verdict = "no-conforme", cause = "BUG-01" }
"#;

    fn a_report(catalogue: &[Check]) -> (tempfile::TempDir, Dossier) {
        let dir = tempfile::tempdir().unwrap();
        let mut dossier = Dossier::open(
            &dir.path().join("dossier.json"),
            "/usr/bin/autofirma",
            Profile::Autofirma,
            catalogue,
            Some(HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                subject_version: "1.9.2".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2:/m.so".to_owned(),
            }),
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
                Verdict::Compliant,
                Some("guardó".to_owned()),
                Duration::from_millis(1_500),
            )
            .unwrap();
        (dir, dossier)
    }

    fn the_json_of(snapshot: &Snapshot) -> Value {
        serde_json::to_value(snapshot).unwrap()
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

    #[test]
    fn the_summary_counts_what_was_observed_and_how_it_fell_against_the_baseline() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report(&catalogue);
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(
            &catalogue,
            None,
            &[],
            Some(("tanda", &dossier)),
            &[],
            &activity,
        ));

        assert_eq!(json["report"]["name"], "tanda");
        assert_eq!(
            json["report"]["summary"],
            json!({"total": 3, "compliant": 2, "noncompliant": 0, "not_observable": 0, "pending": 1})
        );
        assert_eq!(
            json["report"]["baseline"],
            json!({"matching": 1, "surprises": 1, "unmeasured": 0, "pending": 1})
        );
        assert_eq!(json["report"]["green"], false);
        assert_eq!(
            json["report"]["surprises"],
            json!(["a_save: se esperaba NO CONFORME y salió CONFORME"])
        );
        assert_eq!(json["report"]["resolved"], 2);
        assert_eq!(json["report"]["total"], 3);
    }

    #[test]
    fn a_resolved_check_carries_its_verdict_its_expectation_and_the_contrast() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report(&catalogue);
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(
            &catalogue,
            None,
            &[],
            Some(("tanda", &dossier)),
            &[],
            &activity,
        ));
        let save = the_check(&json, "a_save");

        assert_eq!(save["state"], "CONFORME");
        assert_eq!(save["observation"], "guardó");
        assert_eq!(save["duration_ms"], 1_500);
        assert_eq!(
            save["expectation"],
            json!({"verdict": "NO CONFORME", "cause": "BUG-01", "note": null})
        );
        assert_eq!(save["contrast"], "SORPRESA");
        assert_eq!(the_check(&json, "a_signature")["state"], "PENDIENTE");
        assert_eq!(the_check(&json, "a_signature")["contrast"], Value::Null);
    }

    #[test]
    fn the_running_check_the_queued_ones_and_the_question_are_marked() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report(&catalogue);
        let running = ["a_signature".to_owned()];
        let activity = Activity {
            running: &running,
            running_for: Duration::from_millis(12_000),
            queued: vec!["a_save"],
            question: Some(("a_signature", "¿se pidió el PIN? [s/n]", "verdict")),
            ..Activity::default()
        };

        let json = the_json_of(&snapshot_of(
            &catalogue,
            None,
            &[],
            Some(("tanda", &dossier)),
            &[],
            &activity,
        ));

        assert_eq!(
            json["running"],
            json!({"ids": ["a_signature"], "elapsed_ms": 12_000})
        );
        assert_eq!(json["queued"], json!(["a_save"]));
        assert_eq!(
            json["question"],
            json!({"check": "a_signature", "prompt": "¿se pidió el PIN? [s/n]", "kind": "verdict"})
        );
        assert_eq!(the_check(&json, "a_signature")["activity"], "asking");
        assert_eq!(the_check(&json, "a_save")["activity"], "queued");
        assert_eq!(the_check(&json, "a_greeting")["activity"], Value::Null);
    }

    #[test]
    fn the_catalogue_is_grouped_by_suite_in_its_own_order() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(&catalogue, None, &[], None, &[], &activity));

        let suites: Vec<&str> = json["suites"]
            .as_array()
            .unwrap()
            .iter()
            .map(|suite| suite["name"].as_str().unwrap())
            .collect();
        assert_eq!(suites, ["saludo", "operaciones"]);
        assert_eq!(json["report"], Value::Null);
        assert_eq!(the_check(&json, "a_save")["state"], "PENDIENTE");
    }

    #[test]
    fn a_pending_check_says_why_it_did_not_run_and_a_resolved_one_does_not() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_report(&catalogue);
        let reasons = BTreeMap::from([
            (
                "a_signature".to_owned(),
                "se descartó la pregunta".to_owned(),
            ),
            ("a_save".to_owned(), "una razón vieja".to_owned()),
        ]);
        let activity = Activity {
            reasons: Some(&reasons),
            ..Activity::default()
        };

        let json = the_json_of(&snapshot_of(
            &catalogue,
            None,
            &[],
            Some(("tanda", &dossier)),
            &[],
            &activity,
        ));

        assert_eq!(
            the_check(&json, "a_signature")["why_pending"],
            "se descartó la pregunta"
        );
        assert_eq!(the_check(&json, "a_save")["why_pending"], Value::Null);
    }
}
