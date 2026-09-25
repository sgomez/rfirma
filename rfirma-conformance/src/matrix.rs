//! La matriz de happy paths, leída de `catalogue/matrix.toml`: qué celdas se exigen y el estado de cada una, no cómo se miden.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;

use crate::catalogue::{the_catalogue_dir, Check};
use crate::client::Store;
use crate::judge::Expectation;

pub(crate) const THE_MATRIX_FILE: &str = "matrix.toml";

/// Lo que escribe un plano para recorrer todos los valores de un eje.
const EVERY_VALUE: &str = "*";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    origin: Point,
    operation: Vec<Operation>,
    format: Vec<Format>,
    channel: Vec<String>,
    key: Vec<Key>,
    plane: Vec<Plane>,
    cell: Vec<Cell>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    name: String,
    /// Si lo que devuelve es una firma, que la celda tiene que ver verificada.
    signs: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Format {
    name: String,
    /// La condición que verifica con la clave del certificado lo que firmó el cliente.
    verified_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Key {
    name: String,
    store: Store,
}

/// Una celda: un valor en cada eje.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields)]
struct Point {
    operation: String,
    format: String,
    channel: String,
    key: String,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} × {} × {} × {}",
            self.operation, self.format, self.channel, self.key
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Span {
    Every(String),
    These(Vec<String>),
}

/// Las celdas que se exigen: los ejes que nombra, recorridos; los demás, en el origen.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plane {
    operation: Option<Span>,
    format: Option<Span>,
    channel: Option<Span>,
    key: Option<Span>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum State {
    Covered(String),
    NotApplicable(String),
    Gap(String),
}

#[derive(Debug, Deserialize)]
struct Cell {
    operation: Option<String>,
    format: Option<String>,
    channel: Option<String>,
    key: Option<String>,
    #[serde(flatten)]
    state: State,
}

impl Matrix {
    fn axes(&self) -> [(&'static str, Vec<&str>); 4] {
        [
            (
                "operation",
                self.operation.iter().map(|it| it.name.as_str()).collect(),
            ),
            (
                "format",
                self.format.iter().map(|it| it.name.as_str()).collect(),
            ),
            ("channel", self.channel.iter().map(String::as_str).collect()),
            ("key", self.key.iter().map(|it| it.name.as_str()).collect()),
        ]
    }

    fn point_of(&self, cell: &Cell) -> Point {
        let origin = &self.origin;
        let or = |value: &Option<String>, fallback: &String| {
            value.clone().unwrap_or_else(|| fallback.clone())
        };
        Point {
            operation: or(&cell.operation, &origin.operation),
            format: or(&cell.format, &origin.format),
            channel: or(&cell.channel, &origin.channel),
            key: or(&cell.key, &origin.key),
        }
    }

    /// Las celdas que exige algún plano.
    fn the_required_points(&self) -> BTreeSet<Point> {
        let axes = self.axes();
        let origin = [
            &self.origin.operation,
            &self.origin.format,
            &self.origin.channel,
            &self.origin.key,
        ];
        let mut required = BTreeSet::new();
        for plane in &self.plane {
            let spans = [&plane.operation, &plane.format, &plane.channel, &plane.key];
            let values: Vec<Vec<String>> = spans
                .iter()
                .zip(&axes)
                .zip(origin)
                .map(|((span, (_, all)), at)| match span {
                    None => vec![at.clone()],
                    Some(Span::Every(_)) => all.iter().map(|value| (*value).to_owned()).collect(),
                    Some(Span::These(these)) => these.clone(),
                })
                .collect();
            for operation in &values[0] {
                for format in &values[1] {
                    for channel in &values[2] {
                        for key in &values[3] {
                            required.insert(Point {
                                operation: operation.clone(),
                                format: format.clone(),
                                channel: channel.clone(),
                                key: key.clone(),
                            });
                        }
                    }
                }
            }
        }
        required
    }

    fn complaints_about_the_values(&self) -> Vec<String> {
        let axes = self.axes();
        let unknown = |place: &str, axis: usize, value: &str| {
            (!axes[axis].1.contains(&value))
                .then(|| format!("{place}: «{value}» no es un valor de {}", axes[axis].0))
        };
        let points = std::iter::once(("el origen".to_owned(), self.origin.clone())).chain(
            self.cell.iter().map(|cell| {
                let point = self.point_of(cell);
                (format!("la celda {point}"), point)
            }),
        );
        let mut complaints: Vec<String> = points
            .flat_map(|(place, point)| {
                [point.operation, point.format, point.channel, point.key]
                    .into_iter()
                    .enumerate()
                    .filter_map(|(axis, value)| unknown(&place, axis, &value))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (at, plane) in self.plane.iter().enumerate() {
            let place = format!("el plano {}", at + 1);
            let spans = [&plane.operation, &plane.format, &plane.channel, &plane.key];
            for (axis, span) in spans.into_iter().enumerate() {
                match span {
                    Some(Span::Every(every)) if every != EVERY_VALUE => complaints.push(format!(
                        "{place}: «{every}» no es «{EVERY_VALUE}» ni una lista de {}",
                        axes[axis].0
                    )),
                    Some(Span::These(these)) => complaints.extend(
                        these
                            .iter()
                            .filter_map(|value| unknown(&place, axis, value)),
                    ),
                    _ => {}
                }
            }
            if spans.iter().filter(|span| span.is_some()).count() < 2 {
                complaints.push(format!("{place}: no recorre dos ejes"));
            }
        }
        complaints
    }

    fn complaints_about_the_states(&self) -> Vec<String> {
        let required = self.the_required_points();
        let mut declared = BTreeSet::new();
        let mut complaints = Vec::new();
        for cell in &self.cell {
            let point = self.point_of(cell);
            if !required.contains(&point) {
                complaints.push(format!("{point}: la celda no es de ningún plano"));
            }
            if !declared.insert(point.clone()) {
                complaints.push(format!("{point}: la celda está repetida"));
            }
            let said = match &cell.state {
                State::Covered(said) | State::NotApplicable(said) | State::Gap(said) => said,
            };
            if said.trim().is_empty() {
                complaints.push(format!("{point}: el estado de la celda está vacío"));
            }
        }
        complaints.extend(
            required
                .difference(&declared)
                .map(|point| format!("{point}: la celda no tiene estado")),
        );
        complaints
    }

    fn complaints_about_the_coverage(&self, checks: &[Check]) -> Vec<String> {
        let by_id: BTreeMap<&str, &Check> = checks
            .iter()
            .map(|check| (check.id.as_str(), check))
            .collect();
        let mut covering: BTreeMap<&str, Point> = BTreeMap::new();
        let mut complaints = Vec::new();
        for cell in &self.cell {
            let State::Covered(id) = &cell.state else {
                continue;
            };
            let point = self.point_of(cell);
            if let Some(earlier) = covering.insert(id, point.clone()) {
                complaints.push(format!("{point}: {id} ya cubre {earlier}"));
            }
            match by_id.get(id.as_str()) {
                None => complaints.push(format!("{point}: {id} no está en el catálogo")),
                Some(check) => complaints.extend(
                    self.what_does_not_measure(&point, check)
                        .into_iter()
                        .map(|missing| format!("{point}: {id} {missing}")),
                ),
            }
        }
        complaints
    }

    /// Lo que le falta a `check` para medir la celda `point`.
    fn what_does_not_measure(&self, point: &Point, check: &Check) -> Vec<String> {
        let Some(trial) = check.trial() else {
            return vec!["no se conduce".to_owned()];
        };
        let mut missing = Vec::new();
        let drive = &trial.provocation;
        if drive.mode != point.channel {
            missing.push(format!("corre en el modo «{}»", drive.mode));
        }
        let store = self
            .key
            .iter()
            .find(|key| key.name == point.key)
            .map(|key| key.store);
        if store.is_some_and(|store| store != drive.store) {
            missing.push(format!("usa el almacén «{}»", drive.store.name()));
        }
        if !matches!(trial.expects, Expectation::Completes(_)) {
            missing.push("no espera que el trámite se complete".to_owned());
        }
        let signs = self
            .operation
            .iter()
            .any(|operation| operation.name == point.operation && operation.signs);
        let verified_by = self
            .format
            .iter()
            .find(|format| format.name == point.format)
            .map(|format| format.verified_by.as_str());
        if let Some(condition) = verified_by.filter(|_| signs) {
            if !trial.expects.conditions().iter().any(|it| it == condition) {
                missing.push(format!("no espera la condición «{condition}»"));
            }
        }
        missing
    }

    fn complaints_against(&self, checks: &[Check]) -> Vec<String> {
        [
            self.complaints_about_the_values(),
            self.complaints_about_the_states(),
            self.complaints_about_the_coverage(checks),
        ]
        .concat()
    }
}

fn the_matrix_in(raw: &str) -> Result<Matrix, String> {
    toml::from_str(raw).map_err(|error| format!("no es una matriz válida: {error}"))
}

/// Si la matriz de happy paths da estado a cada celda que exige y cada comprobación que la cubre la
/// mide, o lo que no casa.
pub(crate) fn the_matrix_against(checks: &[Check]) -> Result<(), String> {
    let path = the_catalogue_dir().join(THE_MATRIX_FILE);
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} no se pudo leer: {error}", path.display()))?;
    let matrix = the_matrix_in(&raw).map_err(|error| format!("{}: {error}", path.display()))?;
    let complaints = matrix.complaints_against(checks);
    if complaints.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "la matriz de happy paths no casa con el catálogo:\n  {}",
            complaints.join("\n  ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::{read_the_catalogue, the_catalogue_in};

    const AXES: &str = r#"
origin = { operation = "sign", format = "CAdES", channel = "v4", key = "rsa" }
channel = ["v4", "v3"]

[[operation]]
name = "sign"
signs = true

[[operation]]
name = "save"
signs = false

[[format]]
name = "CAdES"
verified_by = "the-signature-verifies"

[[format]]
name = "CAdEStri"
verified_by = "the-presignature-signed-with-the-key"

[[key]]
name = "rsa"
store = "rsa"

[[key]]
name = "ec"
store = "ec"

[[plane]]
operation = "*"
channel = "*"
"#;

    const THE_CELLS: &str = r#"
[[cell]]
covered = "a_signature"

[[cell]]
channel = "v3"
gap = "Nadie firma por v3."

[[cell]]
operation = "save"
covered = "a_save"

[[cell]]
operation = "save"
channel = "v3"
not_applicable = "Por motivos de prueba."
"#;

    fn a_check(id: &str, mode: &str, store: &str, expects: &str) -> String {
        format!(
            r#"
[[check]]
id = "{id}"
set = "firma"
citation = "A.java:1"
statement = "Algo."

[check.drive]
mode = "{mode}"
script = "signcades"
store = "{store}"
act.consent = "Elige uno."
{expects}
"#
        )
    }

    fn the_checks() -> Vec<Check> {
        let raw = [
            a_check(
                "a_signature",
                "v4",
                "rsa",
                r#"expects.completes.conditions = ["the-signature-verifies"]"#,
            ),
            a_check("a_save", "v4", "rsa", "expects.completes = {}"),
        ]
        .concat();
        the_catalogue_in(&raw).unwrap()
    }

    fn complaints_about(axes_and_cells: &str, checks: &[Check]) -> Vec<String> {
        the_matrix_in(axes_and_cells)
            .unwrap()
            .complaints_against(checks)
    }

    #[test]
    fn the_matrix_of_the_repository_gives_a_state_to_every_cell_and_each_covering_check_measures_it(
    ) {
        let checks = read_the_catalogue().unwrap();
        assert_eq!(the_matrix_against(&checks), Ok(()));
    }

    #[test]
    fn a_matrix_with_every_cell_measured_has_no_complaint() {
        let complaints = complaints_about(&[AXES, THE_CELLS].concat(), &the_checks());
        assert!(complaints.is_empty(), "{complaints:?}");
    }

    #[test]
    fn a_cell_of_a_plane_without_a_state_is_named() {
        let without_the_last = THE_CELLS.rsplit_once("[[cell]]").unwrap().0;
        assert_eq!(
            complaints_about(&[AXES, without_the_last].concat(), &the_checks()),
            vec!["save × CAdES × v3 × rsa: la celda no tiene estado"]
        );
    }

    #[test]
    fn a_repeated_cell_and_one_outside_every_plane_are_refused() {
        let extra = "[[cell]]\nchannel = \"v3\"\ngap = \"Otra vez.\"\n\n[[cell]]\nkey = \"ec\"\ngap = \"Fuera.\"\n";
        assert_eq!(
            complaints_about(&[AXES, THE_CELLS, extra].concat(), &the_checks()),
            vec![
                "sign × CAdES × v3 × rsa: la celda está repetida",
                "sign × CAdES × v4 × ec: la celda no es de ningún plano",
            ]
        );
    }

    #[test]
    fn a_value_outside_its_axis_is_refused() {
        let extra = "[[cell]]\nformat = \"NoSuchFormat\"\ngap = \"No existe.\"\n";
        let complaints = complaints_about(&[AXES, THE_CELLS, extra].concat(), &the_checks());
        assert!(complaints.contains(
            &"la celda sign × NoSuchFormat × v4 × rsa: «NoSuchFormat» no es un valor de format"
                .to_owned()
        ));
    }

    #[test]
    fn a_covering_check_that_is_not_in_the_catalogue_is_refused() {
        let cells = THE_CELLS.replace("covered = \"a_save\"", "covered = \"no_such_check\"");
        assert_eq!(
            complaints_about(&[AXES, &cells].concat(), &the_checks()),
            vec!["save × CAdES × v4 × rsa: no_such_check no está en el catálogo"]
        );
    }

    #[test]
    fn a_covering_check_of_a_signing_operation_has_to_expect_the_verified_signature() {
        let checks = the_catalogue_in(
            &[
                a_check("a_signature", "v4", "rsa", "expects.completes = {}"),
                a_check("a_save", "v4", "rsa", "expects.completes = {}"),
            ]
            .concat(),
        )
        .unwrap();
        assert_eq!(
            complaints_about(&[AXES, THE_CELLS].concat(), &checks),
            vec![
                "sign × CAdES × v4 × rsa: a_signature no espera la condición «the-signature-verifies»"
            ]
        );
    }

    #[test]
    fn a_covering_check_of_another_channel_store_or_expectation_does_not_measure_the_cell() {
        let checks = the_catalogue_in(
            &[
                a_check("a_signature", "v3", "ec", r#"expects.code = "SAF_03""#),
                a_check("a_save", "v4", "rsa", "expects.completes = {}"),
            ]
            .concat(),
        )
        .unwrap();
        assert_eq!(
            complaints_about(&[AXES, THE_CELLS].concat(), &checks),
            vec![
                "sign × CAdES × v4 × rsa: a_signature corre en el modo «v3»",
                "sign × CAdES × v4 × rsa: a_signature usa el almacén «ec»",
                "sign × CAdES × v4 × rsa: a_signature no espera que el trámite se complete",
                "sign × CAdES × v4 × rsa: a_signature no espera la condición «the-signature-verifies»",
            ]
        );
    }

    #[test]
    fn one_check_covers_a_single_cell() {
        let cells = THE_CELLS.replace("covered = \"a_save\"", "covered = \"a_signature\"");
        let complaints = complaints_about(&[AXES, &cells].concat(), &the_checks());
        assert!(complaints.contains(
            &"save × CAdES × v4 × rsa: a_signature ya cubre sign × CAdES × v4 × rsa".to_owned()
        ));
    }
}
