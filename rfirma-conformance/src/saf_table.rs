//! La tabla SAF × punto de decisión, leída de `saf-table.toml`: una vista sobre las comprobaciones del catálogo, no un conjunto ni una forma de conducirlas.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::Deserialize;

use crate::catalogue::Check;
use crate::judge::{Code, Expectation};

/// Dónde decide el original el código: qué componente lo emite y en qué momento del trámite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionPoint {
    Parser,
    Channel,
    BeforeCertificate,
    KeyStore,
    AfterCertificate,
    BatchPresigner,
    BatchPostsigner,
    TriphaseServer,
}

/// Cómo mide el catálogo una fila: con qué comprobaciones, o por qué no la mide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coverage {
    Covered(Vec<String>),
    Unmeasurable(String),
    Gap(String),
}

/// Un código SAF desde un punto de decisión, con su cobertura.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "DeclaredRow")]
pub struct Row {
    pub code: String,
    pub point: DecisionPoint,
    pub coverage: Coverage,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredRow {
    code: String,
    point: DecisionPoint,
    covered: Option<Vec<String>>,
    unmeasurable: Option<String>,
    gap: Option<String>,
}

impl TryFrom<DeclaredRow> for Row {
    type Error = String;

    fn try_from(declared: DeclaredRow) -> Result<Self, String> {
        if !matches!(Code::try_from(declared.code.clone()), Ok(Code::Saf(_))) {
            return Err(format!("«{}» no es un código SAF_NN", declared.code));
        }
        let coverage = match (declared.covered, declared.unmeasurable, declared.gap) {
            (Some(ids), None, None) if !ids.is_empty() => Coverage::Covered(ids),
            (None, Some(motive), None) if !motive.trim().is_empty() => {
                Coverage::Unmeasurable(motive)
            }
            (None, None, Some(motive)) if !motive.trim().is_empty() => Coverage::Gap(motive),
            _ => {
                return Err(format!(
                    "{}: una fila lleva exactamente uno de covered, unmeasurable o gap, y no vacío",
                    declared.code
                ))
            }
        };
        Ok(Self {
            code: declared.code,
            point: declared.point,
            coverage,
        })
    }
}

impl Row {
    fn covering_ids(&self) -> &[String] {
        match &self.coverage {
            Coverage::Covered(ids) => ids,
            Coverage::Unmeasurable(_) | Coverage::Gap(_) => &[],
        }
    }

    fn names(&self) -> String {
        format!("{} × {:?}", self.code, self.point)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    row: Vec<Row>,
}

fn the_saf_table_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("saf-table.toml")
}

/// Las filas de la tabla, o por qué no se leen: una fila repetida es una queja.
pub fn read_the_saf_table() -> Result<Vec<Row>, String> {
    let path = the_saf_table_file();
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    the_saf_table_in(&raw).map_err(|complaint| format!("{}: {complaint}", path.display()))
}

fn the_saf_table_in(raw: &str) -> Result<Vec<Row>, String> {
    let table: Table =
        toml::from_str(raw).map_err(|error| format!("no es una tabla SAF válida: {error}"))?;
    let repeated = repeated_rows(&table.row);
    if repeated.is_empty() {
        Ok(table.row)
    } else {
        Err(repeated.join("\n  "))
    }
}

fn repeated_rows(rows: &[Row]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    rows.iter()
        .filter(|row| !seen.insert((row.code.clone(), row.point)))
        .map(|row| format!("{}: fila repetida", row.names()))
        .collect()
}

/// Los códigos que tienen al menos una fila.
pub fn the_codes_of(rows: &[Row]) -> BTreeSet<String> {
    rows.iter().map(|row| row.code.clone()).collect()
}

/// Lo que la tabla y el catálogo no comparten: ids que no existen o no miden el código de su
/// fila, comprobaciones de un SAF sin fila o en dos, y dos que miden el mismo rechazo.
pub fn complaints_against(rows: &[Row], checks: &[Check]) -> Vec<String> {
    [
        ids_that_do_not_measure_their_row(rows, checks),
        checks_of_a_saf_without_a_row(rows, checks),
        checks_in_more_than_one_row(rows),
        checks_that_observe_the_same_rejection(rows, checks),
    ]
    .concat()
}

fn the_saf_it_expects(check: &Check) -> Option<&str> {
    match &check.trial()?.expects {
        Expectation::Code(Code::Saf(code)) => Some(code),
        _ => None,
    }
}

fn why_it_does_not_measure(check: &Check, code: &str) -> Option<String> {
    let Some(trial) = check.trial() else {
        return Some("no se conduce".to_owned());
    };
    match &trial.expects {
        Expectation::Code(Code::Saf(expected)) if expected == code => None,
        Expectation::Code(expected) => Some(format!("espera {expected}")),
        _ if names_the_code(&check.the_declared_text(), code) => None,
        _ => Some(format!("no nombra {code}")),
    }
}

fn names_the_code(text: &str, code: &str) -> bool {
    text.match_indices(code)
        .any(|(at, _)| !text[at + code.len()..].starts_with(|next: char| next.is_ascii_digit()))
}

fn ids_that_do_not_measure_their_row(rows: &[Row], checks: &[Check]) -> Vec<String> {
    let by_id: BTreeMap<&str, &Check> = checks
        .iter()
        .map(|check| (check.id.as_str(), check))
        .collect();
    let by_id = &by_id;
    rows.iter()
        .flat_map(|row| {
            row.covering_ids().iter().filter_map(move |id| {
                let why = match by_id.get(id.as_str()) {
                    None => Some("no es una comprobación del catálogo".to_owned()),
                    Some(check) => why_it_does_not_measure(check, &row.code),
                };
                why.map(|why| format!("{}: {id} {why}", row.names()))
            })
        })
        .collect()
}

fn checks_of_a_saf_without_a_row(rows: &[Row], checks: &[Check]) -> Vec<String> {
    let listed: BTreeSet<&str> = rows
        .iter()
        .flat_map(|row| row.covering_ids().iter().map(String::as_str))
        .collect();
    checks
        .iter()
        .filter_map(|check| {
            let code = the_saf_it_expects(check)?;
            (!listed.contains(check.id.as_str()))
                .then(|| format!("{}: espera {code} y no está en ninguna fila", check.id))
        })
        .collect()
}

fn checks_in_more_than_one_row(rows: &[Row]) -> Vec<String> {
    let mut rows_of: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for row in rows {
        for id in row.covering_ids() {
            rows_of.entry(id).or_default().push(row.names());
        }
    }
    rows_of
        .into_iter()
        .filter(|(_, rows)| rows.len() > 1)
        .map(|(id, rows)| format!("{id}: está en {}", rows.join(" y en ")))
        .collect()
}

/// Dos comprobaciones de una fila que esperan su código del mismo guion por el mismo canal
/// observan el mismo rechazo, cambie lo que cambie la persona o el almacén.
fn checks_that_observe_the_same_rejection(rows: &[Row], checks: &[Check]) -> Vec<String> {
    let by_id: BTreeMap<&str, &Check> = checks
        .iter()
        .map(|check| (check.id.as_str(), check))
        .collect();
    rows.iter()
        .flat_map(|row| {
            let mut first_of: BTreeMap<(&str, &str), &str> = BTreeMap::new();
            row.covering_ids()
                .iter()
                .filter_map(|id| by_id.get(id.as_str()).copied())
                .filter(|check| the_saf_it_expects(check).is_some())
                .filter_map(|check| {
                    let provocation = check.provocation()?;
                    let request = (provocation.mode.as_str(), provocation.script.as_str());
                    match first_of.get(&request) {
                        Some(first) => Some(format!(
                            "{}: {} observa el mismo rechazo que {first}",
                            row.names(),
                            check.id
                        )),
                        None => {
                            first_of.insert(request, check.id.as_str());
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::the_catalogue_in;

    fn a_code_check(id: &str, mode: &str, script: &str, code: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"firma\"\nchapter = \"06\"\n\
             citation = \"A.java:1\"\nstatement = \"Algo.\"\n\n\
             [check.drive]\nmode = \"{mode}\"\nscript = \"{script}\"\nexpects.code = \"{code}\"\n\n"
        )
    }

    fn a_condition_check(id: &str, statement: &str) -> String {
        format!(
            "[[check]]\nid = \"{id}\"\nset = \"peticion\"\nchapter = \"02\"\n\
             citation = \"A.java:1\"\nstatement = \"{statement}\"\n\n\
             [check.drive]\nmode = \"v4\"\nscript = \"protocol-v4\"\n\
             expects.completes.conditions = [\"refused\"]\n\n"
        )
    }

    fn checks(raw: &[String]) -> Vec<Check> {
        the_catalogue_in(&raw.concat()).unwrap()
    }

    fn rows(raw: &str) -> Vec<Row> {
        the_saf_table_in(raw).unwrap()
    }

    #[test]
    fn a_row_is_covered_unmeasurable_or_a_declared_gap() {
        let read = rows(
            "[[row]]\ncode = \"SAF_06\"\npoint = \"before_certificate\"\ncovered = [\"a\"]\n\
             [[row]]\ncode = \"SAF_52\"\npoint = \"key_store\"\nunmeasurable = \"Tarjeta de verdad.\"\n\
             [[row]]\ncode = \"SAF_26\"\npoint = \"batch_postsigner\"\ngap = \"Nadie lo provoca.\"\n",
        );

        assert_eq!(
            read.iter()
                .map(|row| row.coverage.clone())
                .collect::<Vec<_>>(),
            vec![
                Coverage::Covered(vec!["a".to_owned()]),
                Coverage::Unmeasurable("Tarjeta de verdad.".to_owned()),
                Coverage::Gap("Nadie lo provoca.".to_owned()),
            ]
        );
    }

    #[test]
    fn a_row_without_a_state_or_with_two_is_refused() {
        for raw in [
            "[[row]]\ncode = \"SAF_06\"\npoint = \"parser\"\n",
            "[[row]]\ncode = \"SAF_06\"\npoint = \"parser\"\ncovered = []\n",
            "[[row]]\ncode = \"SAF_06\"\npoint = \"parser\"\ncovered = [\"a\"]\ngap = \"b\"\n",
        ] {
            assert!(the_saf_table_in(raw).is_err(), "debería rechazar:\n{raw}");
        }
    }

    #[test]
    fn a_row_of_something_that_is_no_saf_code_or_no_point_is_refused() {
        for raw in [
            "[[row]]\ncode = \"CANCEL\"\npoint = \"parser\"\ngap = \"b\"\n",
            "[[row]]\ncode = \"SAF_06\"\npoint = \"somewhere\"\ngap = \"b\"\n",
        ] {
            assert!(the_saf_table_in(raw).is_err(), "debería rechazar:\n{raw}");
        }
    }

    #[test]
    fn a_repeated_row_is_refused() {
        let raw = "[[row]]\ncode = \"SAF_06\"\npoint = \"parser\"\ngap = \"a\"\n\
                   [[row]]\ncode = \"SAF_06\"\npoint = \"parser\"\ngap = \"b\"\n";

        assert!(the_saf_table_in(raw)
            .unwrap_err()
            .contains("SAF_06 × Parser: fila repetida"));
    }

    #[test]
    fn a_table_that_agrees_with_its_catalogue_has_no_complaints() {
        let catalogue = checks(&[
            a_code_check("rejects_the_format", "v4", "unknownformat", "SAF_06"),
            a_condition_check("refuses_the_host", "Se rechaza con SAF_13."),
        ]);
        let table = rows(
            "[[row]]\ncode = \"SAF_06\"\npoint = \"before_certificate\"\ncovered = [\"rejects_the_format\"]\n\
             [[row]]\ncode = \"SAF_13\"\npoint = \"parser\"\ncovered = [\"refuses_the_host\"]\n",
        );

        assert_eq!(complaints_against(&table, &catalogue), Vec::<String>::new());
    }

    #[test]
    fn an_id_that_does_not_exist_or_does_not_measure_its_code_is_caught_and_named() {
        let catalogue = checks(&[
            a_code_check("rejects_the_format", "v4", "unknownformat", "SAF_06"),
            a_condition_check("refuses_the_host", "Se rechaza con SAF_130."),
        ]);
        let table = rows(
            "[[row]]\ncode = \"SAF_13\"\npoint = \"parser\"\n\
             covered = [\"rejects_the_format\", \"refuses_the_host\", \"vanished\"]\n\
             [[row]]\ncode = \"SAF_06\"\npoint = \"before_certificate\"\ncovered = [\"rejects_the_format\"]\n",
        );

        let complaints = complaints_against(&table, &catalogue);

        assert!(
            complaints.contains(&"SAF_13 × Parser: rejects_the_format espera SAF_06".to_owned())
        );
        assert!(
            complaints.contains(&"SAF_13 × Parser: refuses_the_host no nombra SAF_13".to_owned())
        );
        assert!(complaints
            .contains(&"SAF_13 × Parser: vanished no es una comprobación del catálogo".to_owned()));
    }

    #[test]
    fn a_check_of_a_saf_outside_every_row_is_caught_and_named() {
        let catalogue = checks(&[a_code_check(
            "rejects_the_format",
            "v4",
            "unknownformat",
            "SAF_06",
        )]);

        assert_eq!(
            complaints_against(&[], &catalogue),
            vec!["rejects_the_format: espera SAF_06 y no está en ninguna fila"]
        );
    }

    #[test]
    fn a_check_in_two_rows_is_caught_and_named() {
        let catalogue = checks(&[a_condition_check("refuses_the_host", "SAF_13 o SAF_03.")]);
        let table = rows(
            "[[row]]\ncode = \"SAF_13\"\npoint = \"parser\"\ncovered = [\"refuses_the_host\"]\n\
             [[row]]\ncode = \"SAF_03\"\npoint = \"parser\"\ncovered = [\"refuses_the_host\"]\n",
        );

        assert_eq!(
            complaints_against(&table, &catalogue),
            vec!["refuses_the_host: está en SAF_13 × Parser y en SAF_03 × Parser"]
        );
    }

    #[test]
    fn two_checks_of_a_row_that_observe_the_same_rejection_are_caught_and_the_other_channel_is_not()
    {
        let catalogue = checks(&[
            a_code_check("rejects_the_format", "v4", "unknownformat", "SAF_06"),
            a_code_check("rejects_it_again", "v4", "unknownformat", "SAF_06")
                .replace("[check.drive]\n", "[check.drive]\nstore = \"ec\"\n"),
            a_code_check(
                "rejects_it_over_the_socket",
                "service",
                "unknownformat",
                "SAF_06",
            ),
        ]);
        let table = rows(
            "[[row]]\ncode = \"SAF_06\"\npoint = \"before_certificate\"\n\
             covered = [\"rejects_the_format\", \"rejects_it_again\", \"rejects_it_over_the_socket\"]\n",
        );

        assert_eq!(
            complaints_against(&table, &catalogue),
            vec![
                "SAF_06 × BeforeCertificate: rejects_it_again observa el mismo rechazo que \
                 rejects_the_format"
            ]
        );
    }

    #[test]
    fn the_table_of_the_repository_agrees_with_the_catalogue() {
        let table = read_the_saf_table().unwrap_or_else(|complaint| panic!("{complaint}"));
        let catalogue = crate::catalogue::read_the_catalogue()
            .unwrap_or_else(|complaint| panic!("{complaint}"));

        let complaints = complaints_against(&table, &catalogue);

        assert!(
            complaints.is_empty(),
            "la tabla SAF no casa con el catálogo:\n  {}",
            complaints.join("\n  ")
        );
    }
}
