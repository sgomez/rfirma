//! La comparación de dos informes: qué resultado dio cada uno en cada comprobación del catálogo,
//! en su orden y con su conjunto, y si sus peticiones coinciden, sin juzgar a ningún cliente.

use serde::Serialize;
use ts_rs::TS;

use crate::catalogue::Check;
use crate::client::ClientKind;
use crate::errand::{ErrandKey, ReceivedRequest};
use crate::outcome::{result_name, ResultName};
use crate::report::Report;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct Comparison {
    a: Side,
    b: Side,
    rows: Vec<Row>,
    differing: usize,
    requests_differing: usize,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
struct Side {
    client: String,
    #[ts(as = "ClientKind")]
    kind: &'static str,
    client_version: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
struct Row {
    id: String,
    set: String,
    chapter: String,
    citation: String,
    #[ts(as = "ResultName")]
    a: &'static str,
    #[ts(as = "ResultName")]
    b: &'static str,
    differ: bool,
    requests: RequestsComparison,
    a_requests: Option<Vec<ReceivedRequest>>,
    b_requests: Option<Vec<ReceivedRequest>>,
}

/// Si las peticiones de los dos trámites coinciden como multiconjunto, dé cada uno lo que dé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
enum RequestsComparison {
    Match,
    Differ,
    NotComparable,
}

impl RequestsComparison {
    fn of(a: Option<&[ReceivedRequest]>, b: Option<&[ReceivedRequest]>) -> Self {
        match (a, b) {
            (Some(a), Some(b)) if as_a_multiset(a) == as_a_multiset(b) => Self::Match,
            (Some(_), Some(_)) => Self::Differ,
            _ => Self::NotComparable,
        }
    }
}

fn as_a_multiset(requests: &[ReceivedRequest]) -> Vec<&ReceivedRequest> {
    let mut sorted: Vec<_> = requests.iter().collect();
    sorted.sort();
    sorted
}

fn the_requests_in(report: &Report, check: &Check) -> Option<Vec<ReceivedRequest>> {
    let key = ErrandKey::of(check)?;
    report.observed(&key)?.outcome.requests.clone()
}

pub(crate) fn compare(left: &Report, right: &Report, catalogue: &[Check]) -> Comparison {
    let rows: Vec<Row> = catalogue
        .iter()
        .map(|check| {
            let a = result_name(left.state_of(&check.id));
            let b = result_name(right.state_of(&check.id));
            let a_requests = the_requests_in(left, check);
            let b_requests = the_requests_in(right, check);
            Row {
                id: check.id.clone(),
                set: check.requirement.set.clone(),
                chapter: check.requirement.chapter.clone(),
                citation: check.requirement.citation.clone(),
                a,
                b,
                differ: a != b,
                requests: RequestsComparison::of(a_requests.as_deref(), b_requests.as_deref()),
                a_requests,
                b_requests,
            }
        })
        .collect();
    Comparison {
        a: side_of(left),
        b: side_of(right),
        differing: rows.iter().filter(|row| row.differ).count(),
        requests_differing: rows
            .iter()
            .filter(|row| row.requests == RequestsComparison::Differ)
            .count(),
        rows,
    }
}

fn side_of(report: &Report) -> Side {
    Side {
        client: report.client().to_owned(),
        kind: report.kind().name(),
        client_version: report.header().client_version.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::checks::Settlement;
    use crate::client::ClientKind;
    use crate::errand::fake::RecordedRunner;
    use crate::errand::{
        the_recorded, ErrandOutcome, ErrandRunner, ObservedErrand, RemoteService, RequestHeaders,
    };
    use crate::outcome::Outcome;
    use crate::report::HeaderCoordinates;
    use crate::witness::fake::FakeWitness;
    use crate::witness::Witness;
    use crate::Probe;

    const A_REMOTE_BATCH: &str = r#"
[[check]]
id = "a_remote_batch"
set = "lote"
chapter = "17"
citation = "BatchSigner.java:340-443"
statement = "Un lote remoto."

[check.drive]
mode = "v4"
script = "batch"
expects.completes.conditions = ["through-both-servlets"]
"#;

    const A_TRIPHASE_SIGNATURE: &str = r#"
[[check]]
id = "a_triphase_signature"
set = "operaciones"
chapter = "16"
citation = "AOCAdESTriPhaseSigner.java:239-356"
statement = "Una firma trifásica."

[check.drive]
mode = "v4"
script = "sign"
expects.completes.conditions = ["through-the-triphase-server"]
"#;

    const THREE_CHECKS: &str = r#"
[[check]]
id = "z_one"
set = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Uno."

[check.drive]
mode = "v4"
script = "selectcert"
expects.completes = {}

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Dos."

[check.drive]
mode = "v4"
script = "selectcert"
expects.completes = {}

[[check]]
id = "m_three"
set = "operaciones"
chapter = "16"
citation = "SignOperation.java:12"
statement = "Tres."

[check.drive]
mode = "v4"
script = "sign"
expects.completes = {}
"#;

    fn a_blank_report(kind: ClientKind, catalogue: &[Check]) -> Report {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        Report::create(
            &path,
            kind.name(),
            kind,
            catalogue,
            HeaderCoordinates {
                os: "linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.9.2".to_owned(),
            },
        )
        .unwrap()
    }

    fn a_report(kind: ClientKind, catalogue: &[Check], outcome: Outcome) -> Report {
        let mut report = a_blank_report(kind, catalogue);
        report
            .resolve("z_one", outcome, None, Duration::ZERO)
            .unwrap();
        report
            .resolve("a_two", Outcome::Compliant, None, Duration::ZERO)
            .unwrap();
        report
    }

    fn a_row(
        id: &str,
        set: &str,
        a: &'static str,
        b: &'static str,
    ) -> (String, String, &'static str, &'static str, bool) {
        (id.to_owned(), set.to_owned(), a, b, a != b)
    }

    #[test]
    fn the_rows_follow_the_catalogue_each_with_its_set_and_mark_the_difference() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let left = a_report(ClientKind::Autofirma, &catalogue, Outcome::Noncompliant);
        let right = a_report(ClientKind::Rfirma, &catalogue, Outcome::Compliant);

        let comparison = compare(&left, &right, &catalogue);

        assert_eq!(comparison.a.kind, "autofirma");
        assert_eq!(comparison.b.client_version, "1.9.2");
        let rows: Vec<_> = comparison
            .rows
            .iter()
            .map(|row| (row.id.clone(), row.set.clone(), row.a, row.b, row.differ))
            .collect();
        assert_eq!(
            rows,
            [
                a_row("z_one", "errores", "NO CONFORME", "CONFORME"),
                a_row("a_two", "errores", "CONFORME", "CONFORME"),
                a_row("m_three", "operaciones", "PENDIENTE", "PENDIENTE"),
            ]
        );
        assert_eq!(
            comparison.rows[0].citation,
            "ProtocolInvocationLauncher.java:741"
        );
        assert_eq!(comparison.differing, 1);
    }

    #[test]
    fn a_check_missing_from_one_report_counts_as_pending() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let left = a_report(ClientKind::Autofirma, &catalogue[..2], Outcome::Compliant);
        let mut right = a_report(ClientKind::Rfirma, &catalogue, Outcome::Compliant);
        right
            .resolve("m_three", Outcome::Compliant, None, Duration::ZERO)
            .unwrap();

        let comparison = compare(&left, &right, &catalogue);

        let missing = comparison
            .rows
            .iter()
            .find(|row| row.id == "m_three")
            .unwrap();
        assert_eq!((missing.a, missing.b), ("PENDIENTE", "CONFORME"));
        assert!(missing.differ);
    }

    fn a_report_replaying(kind: ClientKind, catalogue: &[Check], recorded: &str) -> Report {
        a_report_replaying_the_script(kind, catalogue, "batch", recorded)
    }

    fn a_report_replaying_the_script(
        kind: ClientKind,
        catalogue: &[Check],
        script: &str,
        recorded: &str,
    ) -> Report {
        let mut report = a_blank_report(kind, catalogue);
        let runner = RecordedRunner::replaying(&[(script, the_recorded(recorded))]);
        let probe = Probe {
            client: PathBuf::from("/nowhere/launch-subject"),
            trust_root: PathBuf::from("/nowhere/root.pem"),
            report: tempfile::tempdir().unwrap().keep(),
            patience: Duration::from_secs(1),
            witness: Arc::new(FakeWitness::default()) as Arc<dyn Witness>,
            runner: runner as Arc<dyn ErrandRunner>,
        };
        let settled = probe.run_group(&[&catalogue[0]], None, None);
        let key = ErrandKey::of(&catalogue[0]).unwrap();
        report.observe(key, settled.observed.unwrap()).unwrap();
        for settlement in settled.settlements {
            if let Settlement::Resolved {
                id,
                outcome,
                observation,
                duration,
            } = settlement
            {
                report.resolve(&id, outcome, observation, duration).unwrap();
            }
        }
        report
    }

    fn a_report_observing(
        kind: ClientKind,
        catalogue: &[Check],
        requests: Option<Vec<ReceivedRequest>>,
    ) -> Report {
        let mut report = a_blank_report(kind, catalogue);
        let observed = ObservedErrand {
            outcome: ErrandOutcome {
                launched: true,
                requests,
                ..ErrandOutcome::default()
            },
            transcribed_in: catalogue[0].id.clone(),
            duration_ms: 0,
        };
        report
            .observe(ErrandKey::of(&catalogue[0]).unwrap(), observed)
            .unwrap();
        report
    }

    fn a_request(service: RemoteService, body: &[&str]) -> ReceivedRequest {
        ReceivedRequest {
            service,
            method: "POST".to_owned(),
            path: "/batch".to_owned(),
            query: Vec::new(),
            body: body.iter().map(|name| (*name).to_owned()).collect(),
            content_type: Some("application/x-www-form-urlencoded".to_owned()),
            headers: RequestHeaders {
                origin: None,
                authorization: None,
                accept: None,
            },
        }
    }

    #[test]
    fn the_batch_in_the_query_against_the_batch_in_the_body_differs_with_the_same_result() {
        let catalogue = the_catalogue_in(A_REMOTE_BATCH).unwrap();
        let left = a_report_replaying(
            ClientKind::Autofirma,
            &catalogue,
            "a-remote-batch-with-its-parameters-in-the-body",
        );
        let right = a_report_replaying(
            ClientKind::Rfirma,
            &catalogue,
            "a-remote-batch-with-its-parameters-in-the-query",
        );

        let comparison = compare(&left, &right, &catalogue);

        let row = &comparison.rows[0];
        assert_eq!((row.a, row.b, row.differ), ("CONFORME", "CONFORME", false));
        assert_eq!(row.requests, RequestsComparison::Differ);
        assert_eq!(row.a_requests.as_ref().unwrap()[0].body, ["certs", "json"]);
        assert_eq!(row.b_requests.as_ref().unwrap()[0].query, ["certs", "json"]);
        assert_eq!(
            (comparison.differing, comparison.requests_differing),
            (0, 1)
        );
    }

    #[test]
    fn the_triphase_signature_in_the_query_against_the_body_differs_with_the_same_result() {
        let catalogue = the_catalogue_in(A_TRIPHASE_SIGNATURE).unwrap();
        let left = a_report_replaying_the_script(
            ClientKind::Autofirma,
            &catalogue,
            "sign",
            "a-triphase-signature-with-its-parameters-in-the-body",
        );
        let right = a_report_replaying_the_script(
            ClientKind::Rfirma,
            &catalogue,
            "sign",
            "a-triphase-signature-with-its-parameters-in-the-query",
        );

        let comparison = compare(&left, &right, &catalogue);

        let row = &comparison.rows[0];
        assert_eq!((row.a, row.b, row.differ), ("CONFORME", "CONFORME", false));
        assert_eq!(row.requests, RequestsComparison::Differ);
        assert_eq!(
            row.a_requests.as_ref().unwrap()[0].service,
            RemoteService::Triphase
        );
        assert_eq!(
            row.a_requests.as_ref().unwrap()[0].body,
            ["cert", "cop", "doc", "format", "op"]
        );
        assert_eq!(
            row.b_requests.as_ref().unwrap()[0].query,
            ["cert", "cop", "doc", "format", "op"]
        );
    }

    #[test]
    fn the_same_requests_in_another_order_match() {
        let catalogue = the_catalogue_in(A_REMOTE_BATCH).unwrap();
        let presign = a_request(RemoteService::Presigner, &["certs", "json"]);
        let postsign = a_request(RemoteService::Postsigner, &["certs", "json", "tridata"]);
        let left = a_report_observing(
            ClientKind::Autofirma,
            &catalogue,
            Some(vec![presign.clone(), postsign.clone()]),
        );
        let right = a_report_observing(
            ClientKind::Rfirma,
            &catalogue,
            Some(vec![postsign, presign]),
        );

        let comparison = compare(&left, &right, &catalogue);

        assert_eq!(comparison.rows[0].requests, RequestsComparison::Match);
    }

    #[test]
    fn a_request_repeated_on_one_side_only_differs() {
        let catalogue = the_catalogue_in(A_REMOTE_BATCH).unwrap();
        let presign = a_request(RemoteService::Presigner, &["certs", "json"]);
        let left = a_report_observing(
            ClientKind::Autofirma,
            &catalogue,
            Some(vec![presign.clone(), presign.clone()]),
        );
        let right = a_report_observing(ClientKind::Rfirma, &catalogue, Some(vec![presign]));

        let comparison = compare(&left, &right, &catalogue);

        assert_eq!(comparison.rows[0].requests, RequestsComparison::Differ);
    }

    #[test]
    fn an_errand_without_its_requests_is_not_comparable() {
        let catalogue = the_catalogue_in(A_REMOTE_BATCH).unwrap();
        let before_counting = a_report_observing(ClientKind::Autofirma, &catalogue, None);
        let counted = a_report_observing(ClientKind::Rfirma, &catalogue, Some(Vec::new()));
        let never_run = a_blank_report(ClientKind::Rfirma, &catalogue);

        for (left, right) in [(&before_counting, &counted), (&counted, &never_run)] {
            let comparison = compare(left, right, &catalogue);

            assert_eq!(
                comparison.rows[0].requests,
                RequestsComparison::NotComparable
            );
        }
    }
}
