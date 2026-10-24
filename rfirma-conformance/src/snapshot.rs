//! El estado de la sesión activa para la página: la vista de su informe más lo que es solo de la
//! sesión —candidatos, cliente, cola y comprobación en curso—.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;
use ts_rs::TS;

use crate::catalogue::Check;
use crate::client::{Candidate, Client, ClientKind};
use crate::outcome::CheckState;
use crate::report::Report;
use crate::report_view::{report_view, ReportView};

/// Lo que está pasando ahora en la sesión, más allá de lo que ya quedó escrito en el informe.
#[derive(Default)]
pub(crate) struct Activity<'a> {
    pub(crate) client: Option<&'a Client>,
    pub(crate) candidates: &'a [Candidate],
    pub(crate) client_complaints: &'a [String],
    pub(crate) resolving_client: bool,
    pub(crate) running: &'a [String],
    pub(crate) running_for: Duration,
    pub(crate) queued: Vec<&'a str>,
    pub(crate) reasons: Option<&'a BTreeMap<String, String>>,
}

/// Un informe del directorio de informes, tal y como se ofrece para elegirlo o compararlo.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub(crate) struct ReportEntry {
    pub(crate) name: String,
    #[ts(as = "Option<ClientKind>")]
    pub(crate) kind: Option<&'static str>,
    pub(crate) client: Option<String>,
    pub(crate) client_version: Option<String>,
    pub(crate) date: Option<String>,
    pub(crate) continuable: bool,
    pub(crate) why_not: Option<String>,
    pub(crate) complaint: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct Snapshot<'a> {
    client: Option<&'a Client>,
    candidates: &'a [Candidate],
    client_complaints: &'a [String],
    resolving_client: bool,
    report_name: Option<&'a str>,
    report: Option<ReportView<'a>>,
    reports: &'a [ReportEntry],
    running: Option<RunningView<'a>>,
    queued: &'a [&'a str],
    why_pending: BTreeMap<&'a str, &'a str>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct RunningView<'a> {
    ids: &'a [String],
    #[ts(type = "number")]
    elapsed_ms: u128,
}

/// El estado de la sesión, con `open` como el informe abierto y su nombre.
pub(crate) fn snapshot_of<'a>(
    catalogue: &'a [Check],
    open: Option<(&'a str, &'a Report)>,
    reports: &'a [ReportEntry],
    activity: &'a Activity<'a>,
) -> Snapshot<'a> {
    let report = open.map(|(_, report)| report);
    Snapshot {
        client: activity.client,
        candidates: activity.candidates,
        client_complaints: activity.client_complaints,
        resolving_client: activity.resolving_client,
        report_name: open.map(|(name, _)| name),
        report: report.map(|report| report_view(report, catalogue)),
        reports,
        running: (!activity.running.is_empty()).then_some(RunningView {
            ids: activity.running,
            elapsed_ms: activity.running_for.as_millis(),
        }),
        queued: &activity.queued,
        why_pending: the_reasons_still_pending(report, activity),
    }
}

fn the_reasons_still_pending<'a>(
    report: Option<&Report>,
    activity: &'a Activity,
) -> BTreeMap<&'a str, &'a str> {
    activity
        .reasons
        .into_iter()
        .flatten()
        .filter(|(id, _)| {
            !matches!(
                report.and_then(|report| report.state_of(id)),
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
    use crate::catalogue::the_catalogue_in;
    use crate::client::{ClientKind, Origin};
    use crate::outcome::Outcome;
    use crate::report::HeaderCoordinates;

    const TWO_CHECKS: &str = r#"
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

[check.drive]
mode = "v4"
script = "save"
expects.completes = {}
"#;

    fn a_report(catalogue: &[Check]) -> (tempfile::TempDir, Report) {
        let dir = tempfile::tempdir().unwrap();
        let mut report = Report::create(
            &dir.path().join("dossier.json"),
            "/usr/bin/autofirma",
            ClientKind::Autofirma,
            Origin::Installed,
            catalogue,
            HeaderCoordinates {
                os: "Linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.9.2".to_owned(),
                scope: None,
            },
        )
        .unwrap();
        report
            .resolve("a_save", Outcome::Compliant, None, Duration::ZERO)
            .unwrap();
        (dir, report)
    }

    fn the_json_of(snapshot: &Snapshot) -> Value {
        serde_json::to_value(snapshot).unwrap()
    }

    #[test]
    fn the_session_carries_the_view_of_its_report_under_its_name() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
        let (_dir, report) = a_report(&catalogue);
        let activity = Activity::default();

        let json = the_json_of(&snapshot_of(
            &catalogue,
            Some(("informe", &report)),
            &[],
            &activity,
        ));

        assert_eq!(json["report_name"], "informe");
        assert_eq!(
            json["report"],
            serde_json::to_value(report_view(&report, &catalogue)).unwrap()
        );
    }

    #[test]
    fn the_running_check_and_the_queue_travel_apart_from_the_report() {
        let catalogue = the_catalogue_in(TWO_CHECKS).unwrap();
        let (_dir, report) = a_report(&catalogue);
        let running = ["a_signature".to_owned()];
        let activity = Activity {
            running: &running,
            running_for: Duration::from_millis(12_000),
            queued: vec!["a_save"],
            ..Activity::default()
        };

        let json = the_json_of(&snapshot_of(
            &catalogue,
            Some(("informe", &report)),
            &[],
            &activity,
        ));

        assert_eq!(
            json["running"],
            json!({"ids": ["a_signature"], "elapsed_ms": 12_000})
        );
        assert_eq!(json["queued"], json!(["a_save"]));
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
        let (_dir, report) = a_report(&catalogue);
        let reasons = BTreeMap::from([
            (
                "a_signature".to_owned(),
                "se saltó antes de empezar".to_owned(),
            ),
            ("a_save".to_owned(), "una razón vieja".to_owned()),
        ]);
        let activity = Activity {
            reasons: Some(&reasons),
            ..Activity::default()
        };

        let json = the_json_of(&snapshot_of(
            &catalogue,
            Some(("informe", &report)),
            &[],
            &activity,
        ));

        assert_eq!(
            json["why_pending"],
            json!({"a_signature": "se saltó antes de empezar"})
        );
    }
}
