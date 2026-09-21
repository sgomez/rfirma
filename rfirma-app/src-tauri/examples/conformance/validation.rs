//! La validación de la suite contra una referencia: si la tanda de un cliente conocido da lo que la
//! referencia prevé; no juzga al cliente.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::baseline::{the_verdict_named, verdict_name, PENDING_NAME};
use crate::catalogue::Check;
use crate::dossier::{CheckState, Dossier, Verdict};

/// Un resultado distinto del esperado en un cliente conocido, con la ficha que lo explica.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Known {
    pub id: String,
    #[serde(deserialize_with = "the_verdict_named")]
    pub outcome: Verdict,
    pub cause: String,
    pub note: String,
}

/// Lo que se sabe de antemano de la tanda de un cliente concreto; lo que no nombra, sale CONFORME.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    #[serde(default)]
    pub known: Vec<Known>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub(crate) enum Validation {
    Validated,
    Discrepant { discrepancies: Vec<Discrepancy> },
}

/// Una comprobación medible que no da lo previsto: un fallo de la suite o de la referencia.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Discrepancy {
    pub id: String,
    pub expected: &'static str,
    pub observed: &'static str,
    pub cause: Option<String>,
    pub note: Option<String>,
}

pub(crate) fn validate(
    dossier: &Dossier,
    catalogue: &[Check],
    reference: &Reference,
) -> Validation {
    let discrepancies: Vec<Discrepancy> = catalogue
        .iter()
        .filter(|check| check.unmeasurable.is_none())
        .filter_map(|check| {
            let known = reference.known.iter().find(|known| known.id == check.id);
            let expected = known.map_or(Verdict::Compliant, |known| known.outcome);
            let observed = dossier.state_of(&check.id).unwrap_or(CheckState::Pending);
            (observed != CheckState::Resolved(expected)).then(|| Discrepancy {
                id: check.id.clone(),
                expected: verdict_name(expected),
                observed: match observed {
                    CheckState::Pending => PENDING_NAME,
                    CheckState::Resolved(verdict) => verdict_name(verdict),
                },
                cause: known.map(|known| known.cause.clone()),
                note: known.map(|known| known.note.clone()),
            })
        })
        .collect();
    if discrepancies.is_empty() {
        Validation::Validated
    } else {
        Validation::Discrepant { discrepancies }
    }
}

pub(crate) fn the_reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/conformance/reference")
}

/// Lee la referencia `name` de `dir`, que solo puede ser uno de sus ficheros y nunca una ruta.
pub(crate) fn read_the_reference(dir: &Path, name: &str) -> Result<Reference, String> {
    let path = the_reference_files_in(dir)?
        .into_iter()
        .find(|path| path.file_stem().is_some_and(|stem| stem == name))
        .ok_or_else(|| format!("no hay referencia llamada «{name}»"))?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    toml::from_str(&raw).map_err(|error| format!("{}: {error}", path.display()))
}

/// Los nombres de las referencias de `dir`, en orden alfabético.
pub(crate) fn the_references_in(dir: &Path) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = the_reference_files_in(dir)?
        .iter()
        .filter_map(|path| path.file_stem()?.to_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

fn the_reference_files_in(dir: &Path) -> Result<Vec<PathBuf>, String> {
    Ok(std::fs::read_dir(dir)
        .map_err(|error| format!("{} no se pudo leer: {error}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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
id = "a_save"
suite = "operaciones"
chapter = "16"
citation = "C.java:3"
statement = "Guarda."
drive = { mode = "v4", script = "save" }

[[check]]
id = "an_unmeasurable_one"
suite = "operaciones"
chapter = "16"
citation = "D.java:4"
statement = "No llega al cable."
unmeasurable = "No viaja por el cable."
"#;

    const A_REFERENCE: &str = r#"
[[known]]
id = "a_save"
outcome = "no-conforme"
cause = "BUG-01"
note = "Guarda mal."
"#;

    fn a_run(results: &[(&str, Verdict)], catalogue: &[Check]) -> (tempfile::TempDir, Dossier) {
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
        for (id, verdict) in results {
            dossier.resolve(id, *verdict, None, Duration::ZERO).unwrap();
        }
        (dir, dossier)
    }

    fn a_reference() -> Reference {
        toml::from_str(A_REFERENCE).unwrap()
    }

    #[test]
    fn a_run_that_gives_what_the_reference_foresees_is_validated() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_run(
            &[
                ("a_greeting", Verdict::Compliant),
                ("a_save", Verdict::Noncompliant),
            ],
            &catalogue,
        );

        assert_eq!(
            validate(&dossier, &catalogue, &a_reference()),
            Validation::Validated
        );
    }

    #[test]
    fn a_result_other_than_the_foreseen_one_is_a_discrepancy_naming_both() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_run(
            &[
                ("a_greeting", Verdict::NotObservable),
                ("a_save", Verdict::Compliant),
            ],
            &catalogue,
        );

        assert_eq!(
            validate(&dossier, &catalogue, &a_reference()),
            Validation::Discrepant {
                discrepancies: vec![
                    Discrepancy {
                        id: "a_greeting".to_owned(),
                        expected: "CONFORME",
                        observed: "NO OBSERVABLE",
                        cause: None,
                        note: None,
                    },
                    Discrepancy {
                        id: "a_save".to_owned(),
                        expected: "NO CONFORME",
                        observed: "CONFORME",
                        cause: Some("BUG-01".to_owned()),
                        note: Some("Guarda mal.".to_owned()),
                    },
                ]
            }
        );
    }

    #[test]
    fn a_pending_measurable_check_is_a_discrepancy_and_an_unmeasurable_one_never_counts() {
        let catalogue = the_catalogue_in(THREE_CHECKS).unwrap();
        let (_dir, dossier) = a_run(&[("a_greeting", Verdict::Compliant)], &catalogue);

        let Validation::Discrepant { discrepancies } =
            validate(&dossier, &catalogue, &a_reference())
        else {
            panic!("una comprobación medible pendiente no deja validar la tanda");
        };

        let ids: Vec<&str> = discrepancies.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["a_save"]);
        assert_eq!(discrepancies[0].observed, "PENDIENTE");
    }

    #[test]
    fn the_reference_of_autofirma_1_9_2_only_names_checks_of_the_catalogue() {
        let catalogue = crate::catalogue::read_the_catalogue().unwrap();
        let reference = read_the_reference(&the_reference_dir(), "autofirma-1.9.2").unwrap();

        assert!(!reference.known.is_empty());
        for known in &reference.known {
            assert!(
                catalogue.iter().any(|check| check.id == known.id),
                "{} no está en el catálogo",
                known.id
            );
        }
    }

    #[test]
    fn a_reference_is_a_file_of_its_folder_and_never_a_path() {
        let complaint =
            read_the_reference(&the_reference_dir(), "../catalogue/saludo").unwrap_err();

        assert!(complaint.contains("no hay referencia llamada"));
    }

    #[test]
    fn the_references_are_listed_by_name() {
        let names = the_references_in(&the_reference_dir()).unwrap();

        assert!(names.contains(&"autofirma-1.9.2".to_owned()));
        assert!(names.iter().all(|name| !name.ends_with(".toml")));
    }

    #[test]
    fn an_outcome_outside_the_vocabulary_is_refused() {
        assert!(toml::from_str::<Reference>(
            "[[known]]\nid = \"a\"\noutcome = \"regular\"\ncause = \"BUG-01\"\nnote = \"x\"\n"
        )
        .is_err());
    }
}
