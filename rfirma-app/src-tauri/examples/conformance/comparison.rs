//! La comparación de dos expedientes: qué observó cada sujeto en cada comprobación, sin pasar por
//! la línea base.

use std::collections::BTreeSet;
use std::path::Path;

use crate::baseline::verdict_name;
use crate::dossier::{CheckState, Dossier};
use crate::listing::{chapter_tag, PENDING_BADGE, RED, RESET};

pub(crate) fn compare(a: &Path, b: &Path) -> Result<String, String> {
    let left = Dossier::read(a)?;
    let right = Dossier::read(b)?;
    Ok(format_comparison(&left, &right, false))
}

pub(crate) fn format_comparison(left: &Dossier, right: &Dossier, use_color: bool) -> String {
    let mut out = format!(
        "A: {} ({}, versión {})\nB: {} ({}, versión {})\n",
        left.subject(),
        left.profile().name(),
        left.header().subject_version,
        right.subject(),
        right.profile().name(),
        right.header().subject_version
    );

    let ids: BTreeSet<&str> = left
        .checks()
        .map(|(id, _)| id)
        .chain(right.checks().map(|(id, _)| id))
        .collect();
    let mut differing = 0;
    for id in ids {
        let (observed_left, chapter) = observation_of(left, id);
        let (observed_right, other_chapter) = observation_of(right, id);
        let differ = observed_left != observed_right;
        differing += usize::from(differ);
        let tail = match (differ, use_color) {
            (false, _) => String::new(),
            (true, false) => " ← difieren".to_owned(),
            (true, true) => format!("{RED} ← difieren{RESET}"),
        };
        out.push_str(&format!(
            "\n{} {id}\n  A: {observed_left}\n  B: {observed_right}{tail}\n",
            chapter_tag(chapter.unwrap_or(other_chapter.unwrap_or("--")))
        ));
    }
    out.push_str(&format!("\n{differing} comprobaciones difieren.\n"));
    out
}

fn observation_of<'a>(dossier: &'a Dossier, id: &str) -> (&'static str, Option<&'a str>) {
    match dossier.checks().find(|(each, _)| *each == id) {
        None => ("ausente", None),
        Some((_, record)) => (
            match record.state {
                CheckState::Pending => PENDING_BADGE,
                CheckState::Resolved(verdict) => verdict_name(verdict),
            },
            Some(record.chapter.as_str()),
        ),
    }
}

#[cfg(test)]
mod tests {
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
        dossier.resolve("a_one", verdict, None).unwrap();
        dossier.resolve("a_two", Verdict::Compliant, None).unwrap();
        dossier
    }

    #[test]
    fn the_comparison_shows_what_each_subject_observed_and_marks_the_difference() {
        let left = a_dossier(Profile::Autofirma, Verdict::Noncompliant);
        let right = a_dossier(Profile::Rfirma, Verdict::Compliant);

        let output = format_comparison(&left, &right, false);

        assert!(output.contains("A: autofirma (autofirma, versión 1.9.2)"));
        assert!(output.contains("[Cap. 15] a_one\n  A: NO CONFORME\n  B: CONFORME ← difieren"));
        assert!(output.contains("[Cap. 15] a_two\n  A: CONFORME\n  B: CONFORME\n"));
        assert!(output.contains("1 comprobaciones difieren."));
    }
}
