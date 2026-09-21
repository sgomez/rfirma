//! La comparación de dos informes: qué resultado dio cada uno en cada comprobación del catálogo,
//! en su orden y con su conjunto, sin juzgar a ningún cliente.

use serde::Serialize;
use ts_rs::TS;

use crate::catalogue::Check;
use crate::client::ClientKind;
use crate::outcome::{result_name, ResultName};
use crate::report::Report;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub(crate) struct Comparison {
    a: Side,
    b: Side,
    rows: Vec<Row>,
    differing: usize,
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
}

pub(crate) fn compare(left: &Report, right: &Report, catalogue: &[Check]) -> Comparison {
    let rows: Vec<Row> = catalogue
        .iter()
        .map(|check| {
            let a = result_name(left.state_of(&check.id));
            let b = result_name(right.state_of(&check.id));
            Row {
                id: check.id.clone(),
                set: check.set.clone(),
                chapter: check.chapter.clone(),
                citation: check.citation.clone(),
                a,
                b,
                differ: a != b,
            }
        })
        .collect();
    Comparison {
        a: side_of(left),
        b: side_of(right),
        differing: rows.iter().filter(|row| row.differ).count(),
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
    use std::time::Duration;

    use super::*;
    use crate::catalogue::the_catalogue_in;
    use crate::client::ClientKind;
    use crate::outcome::Outcome;
    use crate::report::HeaderCoordinates;

    const THREE_CHECKS: &str = r#"
[[check]]
id = "z_one"
set = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Uno."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_two"
set = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Dos."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "m_three"
set = "operaciones"
chapter = "16"
citation = "SignOperation.java:12"
statement = "Tres."
drive = { mode = "v4", script = "sign" }
"#;

    fn a_report(kind: ClientKind, catalogue: &[Check], outcome: Outcome) -> Report {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let mut report = Report::create(
            &path,
            kind.name(),
            kind,
            catalogue,
            HeaderCoordinates {
                os: "linux".to_owned(),
                os_version: "6.0".to_owned(),
                client_version: "1.9.2".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2".to_owned(),
            },
        )
        .unwrap();
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
}
