//! La comparación de dos informes: qué resultado dio cada uno en cada comprobación del catálogo,
//! en su orden y con su conjunto, sin juzgar a ningún cliente.

use serde::Serialize;

use crate::catalogue::Check;
use crate::dossier::Dossier;
use crate::report_view::result_name;

#[derive(Debug, Serialize)]
pub(crate) struct Comparison {
    a: Side,
    b: Side,
    rows: Vec<Row>,
    differing: usize,
}

#[derive(Debug, Serialize)]
struct Side {
    subject: String,
    profile: &'static str,
    subject_version: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Row {
    id: String,
    suite: String,
    chapter: String,
    citation: String,
    a: &'static str,
    b: &'static str,
    differ: bool,
}

pub(crate) fn compare(left: &Dossier, right: &Dossier, catalogue: &[Check]) -> Comparison {
    let rows: Vec<Row> = catalogue
        .iter()
        .map(|check| {
            let a = result_name(left.state_of(&check.id));
            let b = result_name(right.state_of(&check.id));
            Row {
                id: check.id.clone(),
                suite: check.suite.clone(),
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

fn side_of(dossier: &Dossier) -> Side {
    Side {
        subject: dossier.subject().to_owned(),
        profile: dossier.profile().name(),
        subject_version: dossier.header().subject_version.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::baseline::Profile;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::{HeaderCoordinates, Verdict};

    const THREE_CHECKS: &str = r#"
[[check]]
id = "z_one"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Uno."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_two"
suite = "errores"
chapter = "15"
citation = "ProtocolInvocationLauncher.java:741"
statement = "Dos."
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "m_three"
suite = "operaciones"
chapter = "16"
citation = "SignOperation.java:12"
statement = "Tres."
drive = { mode = "v4", script = "sign" }
"#;

    fn a_dossier(profile: Profile, catalogue: &[Check], verdict: Verdict) -> Dossier {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let mut dossier = Dossier::create(
            &path,
            profile.name(),
            profile,
            catalogue,
            HeaderCoordinates {
                os: "linux".to_owned(),
                os_version: "6.0".to_owned(),
                subject_version: "1.9.2".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2".to_owned(),
            },
        )
        .unwrap();
        dossier
            .resolve("z_one", verdict, None, Duration::ZERO)
            .unwrap();
        dossier
            .resolve("a_two", Verdict::Compliant, None, Duration::ZERO)
            .unwrap();
        dossier
    }

    fn a_row(
        id: &str,
        suite: &str,
        a: &'static str,
        b: &'static str,
    ) -> (String, String, &'static str, &'static str, bool) {
        (id.to_owned(), suite.to_owned(), a, b, a != b)
    }

    #[test]
    fn the_rows_follow_the_catalogue_each_with_its_suite_and_mark_the_difference() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let left = a_dossier(Profile::Autofirma, &catalogue, Verdict::Noncompliant);
        let right = a_dossier(Profile::Rfirma, &catalogue, Verdict::Compliant);

        let comparison = compare(&left, &right, &catalogue);

        assert_eq!(comparison.a.profile, "autofirma");
        assert_eq!(comparison.b.subject_version, "1.9.2");
        let rows: Vec<_> = comparison
            .rows
            .iter()
            .map(|row| (row.id.clone(), row.suite.clone(), row.a, row.b, row.differ))
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
        let left = a_dossier(Profile::Autofirma, &catalogue[..2], Verdict::Compliant);
        let mut right = a_dossier(Profile::Rfirma, &catalogue, Verdict::Compliant);
        right
            .resolve("m_three", Verdict::Compliant, None, Duration::ZERO)
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
