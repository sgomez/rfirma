//! El estado de la sesión activa para la página: la vista de su informe más lo que es solo de la
//! sesión —cliente, cola, comprobación en curso y pregunta—.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::catalogue::Check;
use crate::dossier::{CheckState, Dossier};
use crate::report_view::{report_view, ReportView};
use crate::subject::Subject;

/// Lo que está pasando ahora en la sesión, más allá de lo que ya quedó escrito en el informe.
#[derive(Default)]
pub(crate) struct Activity<'a> {
    pub(crate) subject: Option<&'a Subject>,
    pub(crate) subject_complaints: &'a [String],
    pub(crate) resolving_subject: bool,
    pub(crate) running: &'a [String],
    pub(crate) running_for: Duration,
    pub(crate) queued: Vec<&'a str>,
    pub(crate) question: Option<(&'a str, &'a str, &'static str)>,
    pub(crate) reasons: Option<&'a BTreeMap<String, String>>,
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
    report_name: Option<&'a str>,
    report: Option<ReportView<'a>>,
    reports: &'a [ReportEntry],
    running: Option<RunningView<'a>>,
    queued: &'a [&'a str],
    question: Option<QuestionView<'a>>,
    why_pending: BTreeMap<&'a str, &'a str>,
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

/// El estado de la sesión, con `report` como el informe abierto y su nombre.
pub(crate) fn snapshot_of<'a>(
    catalogue: &'a [Check],
    report: Option<(&'a str, &'a Dossier)>,
    reports: &'a [ReportEntry],
    activity: &'a Activity<'a>,
) -> Snapshot<'a> {
    let dossier = report.map(|(_, dossier)| dossier);
    Snapshot {
        subject: activity.subject,
        subject_complaints: activity.subject_complaints,
        resolving_subject: activity.resolving_subject,
        report_name: report.map(|(name, _)| name),
        report: dossier.map(|dossier| report_view(dossier, catalogue)),
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
        why_pending: the_reasons_still_pending(dossier, activity),
    }
}

fn the_reasons_still_pending<'a>(
    dossier: Option<&Dossier>,
    activity: &'a Activity,
) -> BTreeMap<&'a str, &'a str> {
    activity
        .reasons
        .into_iter()
        .flatten()
        .filter(|(id, _)| {
            !matches!(
                dossier.and_then(|dossier| dossier.state_of(id)),
                Some(CheckState::Resolved(_))
            )
        })
        .map(|(id, reason)| (id.as_str(), reason.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::baseline::Profile;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::{HeaderCoordinates, Verdict};

    const TWO_CHECKS: &str = r#"
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

    fn a_report(catalogue: &[Check]) -> (tempfile::TempDir, Dossier) {
        let dir = tempfile::tempdir().unwrap();
        let mut dossier = Dossier::create(
            &dir.path().join("dossier.json"),
            "/usr/bin/autofirma",
            Profile::Autofirma,
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
            .resolve("a_save", Verdict::Compliant, None, Duration::ZERO)
            .unwrap();
        (dir, dossier)
    }

    fn the_json_of(snapshot: &Snapshot) -> Value {
        serde_json::to_value(snapshot).unwrap()
    }

    #[test]
    fn the_session_carries_the_view_of_its_report_under_its_name() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
        let (_dir, dossier) = a_report(&catalogue);
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(
            &catalogue,
            Some(("informe", &dossier)),
            &[],
            &activity,
        ));

        assert_eq!(json["report_name"], "informe");
        assert_eq!(
            json["report"],
            serde_json::to_value(report_view(&dossier, &catalogue)).unwrap()
        );
    }

    #[test]
    fn the_running_check_the_queue_and_the_question_travel_apart_from_the_report() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
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
            Some(("informe", &dossier)),
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
    }

    #[test]
    fn without_a_report_there_is_no_view() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(&catalogue, None, &[], &activity));

        assert_eq!(json["report"], Value::Null);
        assert_eq!(json["report_name"], Value::Null);
    }

    #[test]
    fn a_pending_check_says_why_it_did_not_run_and_a_resolved_one_does_not() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
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
            Some(("informe", &dossier)),
            &[],
            &activity,
        ));

        assert_eq!(
            json["why_pending"],
            json!({"a_signature": "se descartó la pregunta"})
        );
    }
}
