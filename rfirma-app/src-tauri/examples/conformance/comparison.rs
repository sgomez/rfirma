//! La comparación de dos expedientes: qué observó cada sujeto en cada comprobación, sin pasar por
//! la línea base.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::baseline::verdict_name;
use crate::dossier::{CheckState, Dossier};

pub(crate) const PENDING_NAME: &str = "PENDIENTE";

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
    chapter: Option<String>,
    a: &'static str,
    b: &'static str,
    differ: bool,
}

pub(crate) fn compare(left: &Dossier, right: &Dossier) -> Comparison {
    let ids: BTreeSet<&str> = left
        .checks()
        .map(|(id, _)| id)
        .chain(right.checks().map(|(id, _)| id))
        .collect();
    let rows: Vec<Row> = ids
        .into_iter()
        .map(|id| {
            let (a, chapter) = observation_of(left, id);
            let (b, other_chapter) = observation_of(right, id);
            Row {
                id: id.to_owned(),
                chapter: chapter.or(other_chapter).map(str::to_owned),
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

fn observation_of<'a>(dossier: &'a Dossier, id: &str) -> (&'static str, Option<&'a str>) {
    match dossier.record_of(id) {
        None => ("ausente", None),
        Some(record) => (
            match record.state {
                CheckState::Pending => PENDING_NAME,
                CheckState::Resolved(verdict) => verdict_name(verdict),
            },
            Some(record.chapter.as_str()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::baseline::Profile;
    use crate::catalogue::the_catalogue_in;
    use crate::dossier::{HeaderCoordinates, Verdict};

    fn a_dossier(profile: Profile, verdict: Verdict) -> Dossier {
        let path = tempfile::NamedTempFile::new().unwrap().path().to_owned();
        let catalogue = the_catalogue_in(
            r#"
[[check]]
id = "a_one"
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
"#,
        )
        .unwrap();
        let mut dossier = Dossier::open(
            &path,
            profile.name(),
            profile,
            &catalogue,
            Some(HeaderCoordinates {
                os: "linux".to_owned(),
                os_version: "6.0".to_owned(),
                subject_version: "1.9.2".to_owned(),
                transport: "websocket".to_owned(),
                store: "softhsm2".to_owned(),
            }),
        )
        .unwrap();
        dossier
            .resolve("a_one", verdict, None, Duration::ZERO)
            .unwrap();
        dossier
            .resolve("a_two", Verdict::Compliant, None, Duration::ZERO)
            .unwrap();
        dossier
    }

    #[test]
    fn the_comparison_shows_what_each_subject_observed_and_marks_the_difference() {
        let left = a_dossier(Profile::Autofirma, Verdict::Noncompliant);
        let right = a_dossier(Profile::Rfirma, Verdict::Compliant);

        let comparison = compare(&left, &right);

        assert_eq!(comparison.a.profile, "autofirma");
        assert_eq!(comparison.b.subject_version, "1.9.2");
        assert_eq!(
            comparison.rows,
            [
                Row {
                    id: "a_one".to_owned(),
                    chapter: Some("15".to_owned()),
                    a: "NO CONFORME",
                    b: "CONFORME",
                    differ: true,
                },
                Row {
                    id: "a_two".to_owned(),
                    chapter: Some("15".to_owned()),
                    a: "CONFORME",
                    b: "CONFORME",
                    differ: false,
                },
            ]
        );
        assert_eq!(comparison.differing, 1);
    }
}
