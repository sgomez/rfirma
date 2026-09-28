//! La comparación de dos informes: qué resultado dio cada uno en cada comprobación del catálogo,
//! en su orden y con su conjunto, y si sus peticiones coinciden, sin juzgar a ningún cliente.

use serde::Serialize;
use ts_rs::TS;

use std::collections::BTreeSet;

use crate::catalogue::Check;
use crate::client::ClientKind;
use crate::errand::{ErrandKey, ReceivedRequest, RemoteService};
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
    request_differences: Vec<RequestDifference>,
}

/// Lo que difiere de un servicio remoto entre las peticiones de los dos lados, ya explicado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
struct RequestDifference {
    service: RemoteService,
    differences: Vec<String>,
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

fn request_differences(
    a: Option<&[ReceivedRequest]>,
    b: Option<&[ReceivedRequest]>,
) -> Vec<RequestDifference> {
    let (Some(a), Some(b)) = (a, b) else {
        return Vec::new();
    };
    let services: BTreeSet<RemoteService> =
        a.iter().chain(b).map(|request| request.service).collect();
    services
        .into_iter()
        .filter_map(|service| {
            let differences = differences_for(service, a, b);
            (!differences.is_empty()).then_some(RequestDifference {
                service,
                differences,
            })
        })
        .collect()
}

fn differences_for(
    service: RemoteService,
    a: &[ReceivedRequest],
    b: &[ReceivedRequest],
) -> Vec<String> {
    let a_group: Vec<_> = a
        .iter()
        .filter(|request| request.service == service)
        .collect();
    let b_group: Vec<_> = b
        .iter()
        .filter(|request| request.service == service)
        .collect();
    if a_group.len() != b_group.len() {
        return vec![format!(
            "número de peticiones: {} frente a {}",
            a_group.len(),
            b_group.len()
        )];
    }
    a_group
        .into_iter()
        .zip(b_group)
        .flat_map(|(x, y)| explain(x, y))
        .collect()
}

fn explain(a: &ReceivedRequest, b: &ReceivedRequest) -> Vec<String> {
    let mut differences = Vec::new();
    if a.method != b.method {
        differences.push(format!("método: {} frente a {}", a.method, b.method));
    }
    if a.path != b.path {
        differences.push(format!("ruta: {} frente a {}", a.path, b.path));
    }
    if a.query != b.query || a.body != b.body {
        differences.push(format!(
            "parámetros: {} frente a {}",
            where_params_travel(&a.query, &a.body),
            where_params_travel(&b.query, &b.body)
        ));
    }
    if a.content_type != b.content_type {
        differences.push(format!(
            "Content-Type: {} frente a {}",
            an_option(&a.content_type),
            an_option(&b.content_type)
        ));
    }
    for (name, a_value, b_value) in [
        ("origin", &a.headers.origin, &b.headers.origin),
        (
            "authorization",
            &a.headers.authorization,
            &b.headers.authorization,
        ),
        ("accept", &a.headers.accept, &b.headers.accept),
    ] {
        if a_value != b_value {
            differences.push(format!(
                "cabecera {name}: {} frente a {}",
                an_option(a_value),
                an_option(b_value)
            ));
        }
    }
    differences
}

fn where_params_travel(query: &[String], body: &[String]) -> String {
    match (query.is_empty(), body.is_empty()) {
        (false, true) => format!("en la query: {}", query.join(", ")),
        (true, false) => format!("en el cuerpo: {}", body.join(", ")),
        (false, false) => format!(
            "en la query: {}; en el cuerpo: {}",
            query.join(", "),
            body.join(", ")
        ),
        (true, true) => "sin parámetros".to_owned(),
    }
}

fn an_option(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("(ninguno)")
}

pub(crate) fn compare(left: &Report, right: &Report, catalogue: &[Check]) -> Comparison {
    let rows: Vec<Row> = catalogue
        .iter()
        .map(|check| {
            let a = result_name(left.state_of(&check.id));
            let b = result_name(right.state_of(&check.id));
            let a_requests = the_requests_in(left, check);
            let b_requests = the_requests_in(right, check);
            let requests = RequestsComparison::of(a_requests.as_deref(), b_requests.as_deref());
            let request_differences = if requests == RequestsComparison::Differ {
                request_differences(a_requests.as_deref(), b_requests.as_deref())
            } else {
                Vec::new()
            };
            Row {
                id: check.id.clone(),
                set: check.requirement.set.clone(),
                chapter: check.requirement.chapter.clone(),
                citation: check.requirement.citation.clone(),
                a,
                b,
                differ: a != b,
                requests,
                request_differences,
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
        let presigner = row
            .request_differences
            .iter()
            .find(|difference| difference.service == RemoteService::Presigner)
            .unwrap();
        assert!(presigner.differences.iter().any(|line| line
            == "parámetros: en el cuerpo: certs, json frente a en la query: certs, json"));
        let postsigner = row
            .request_differences
            .iter()
            .find(|difference| difference.service == RemoteService::Postsigner)
            .unwrap();
        assert!(postsigner
            .differences
            .iter()
            .any(|line| line.contains("tridata")));
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
        assert!(comparison.rows[0].request_differences.is_empty());
    }

    #[test]
    fn a_difference_names_the_method_the_content_type_and_the_header_that_differ() {
        let catalogue = the_catalogue_in(A_REMOTE_BATCH).unwrap();
        let mut a_side = a_request(RemoteService::Presigner, &["certs"]);
        a_side.method = "GET".to_owned();
        let mut b_side = a_request(RemoteService::Presigner, &["certs"]);
        b_side.content_type = None;
        b_side.headers.origin = Some("https://autofirma.local".to_owned());
        let left = a_report_observing(ClientKind::Autofirma, &catalogue, Some(vec![a_side]));
        let right = a_report_observing(ClientKind::Rfirma, &catalogue, Some(vec![b_side]));

        let comparison = compare(&left, &right, &catalogue);

        let differences = &comparison.rows[0].request_differences[0].differences;
        assert!(differences.contains(&"método: GET frente a POST".to_owned()));
        assert!(differences.contains(
            &"Content-Type: application/x-www-form-urlencoded frente a (ninguno)".to_owned()
        ));
        assert!(differences
            .contains(&"cabecera origin: (ninguno) frente a https://autofirma.local".to_owned()));
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
            assert!(comparison.rows[0].request_differences.is_empty());
        }
    }
}
