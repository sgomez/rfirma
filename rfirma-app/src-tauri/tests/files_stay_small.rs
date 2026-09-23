//! Congela el tamaño de los ficheros de producción y de tests contra `files_stay_small.baseline`, para que crecer exija tocar esa lista a mano.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Líneas no vacías admitidas en un fichero de producción antes de aparecer en el baseline.
const PRODUCTION_THRESHOLD: usize = 500;
/// Líneas no vacías admitidas en un fichero de tests antes de aparecer en el baseline.
const TEST_THRESHOLD: usize = 600;

/// Raíces que recorre la guarda, relativas a la raíz del repositorio.
const SCANNED_ROOTS: &[&str] = &[
    "rfirma-app/src-tauri/src",
    "rfirma-app/src-tauri/tests",
    "rfirma-app/src",
];

/// Prefijos fuera de la guarda: datos o ficheros generados, no código escrito a mano.
const EXEMPT_PREFIXES: &[&str] = &["rfirma-app/src/i18n/locales/"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn baseline_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/files_stay_small.baseline")
}

fn has_a_scanned_extension(path: &str) -> bool {
    [".rs", ".ts", ".tsx"]
        .iter()
        .any(|extension| path.ends_with(extension))
}

fn is_exempt(path: &str) -> bool {
    EXEMPT_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

/// Es test todo fichero bajo un directorio `tests/`, llamado `tests.rs`, o terminado en `.test.ts(x)`.
fn is_a_test_file(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    path.split('/').any(|segment| segment == "tests")
        || basename == "tests.rs"
        || basename.ends_with(".test.ts")
        || basename.ends_with(".test.tsx")
}

fn threshold_for(path: &str) -> usize {
    if is_a_test_file(path) {
        TEST_THRESHOLD
    } else {
        PRODUCTION_THRESHOLD
    }
}

fn non_blank_lines(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

/// Los ficheros versionados en alcance, con su número de líneas no vacías.
fn measure_tracked_files(root: &Path) -> BTreeMap<String, usize> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", "--"])
        .args(SCANNED_ROOTS)
        .current_dir(root)
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");

    String::from_utf8(listing.stdout)
        .expect("las rutas deberian ser UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| has_a_scanned_extension(path))
        .filter(|path| !is_exempt(path))
        .map(|path| {
            let text = std::fs::read_to_string(root.join(path))
                .unwrap_or_else(|error| panic!("no se pudo leer {path}: {error}"));
            (path.to_owned(), non_blank_lines(&text))
        })
        .collect()
}

fn parse_baseline(text: &str) -> BTreeMap<String, usize> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (path, lines) = line
                .rsplit_once(' ')
                .unwrap_or_else(|| panic!("cada linea del baseline es «ruta lineas»: {line:?}"));
            let lines = lines
                .parse()
                .unwrap_or_else(|_| panic!("el numero de lineas deberia ser un entero: {line:?}"));
            (path.to_owned(), lines)
        })
        .collect()
}

const WHAT_TO_DO: &str = "\
Un fichero de producción por encima de 500 líneas se separa por la responsabilidad que sobra; \
un fichero de tests por encima de 600 se parte por comportamiento en tests/<comportamiento>.rs \
y sus helpers se sacan a tests/support.rs. Cuando la partición está hecha, files_stay_small.baseline \
se actualiza a mano con el número de líneas resultante.";

/// Las incoherencias entre lo medido ahora y lo que registra el baseline, una por línea.
fn baseline_problems(
    measured: &BTreeMap<String, usize>,
    baseline: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut problems = Vec::new();

    for (path, &lines) in measured {
        if baseline.contains_key(path) {
            continue;
        }
        let threshold = threshold_for(path);
        if lines > threshold {
            problems.push(format!(
                "{path} tiene {lines} líneas (umbral {threshold}) y no está en el baseline"
            ));
        }
    }

    for (path, &recorded) in baseline {
        let threshold = threshold_for(path);
        match measured.get(path) {
            None => problems.push(format!("{path} está en el baseline pero ya no existe")),
            Some(&lines) if lines <= threshold => problems.push(format!(
                "{path} tiene {lines} líneas, por debajo del umbral {threshold}; \
                 sale del baseline"
            )),
            Some(&lines) if lines > recorded => problems.push(format!(
                "{path} creció de {recorded} a {lines} líneas; actualiza el baseline"
            )),
            Some(&lines) if lines < recorded => problems.push(format!(
                "{path} bajó de {recorded} a {lines} líneas sin actualizar el baseline"
            )),
            Some(_) => {}
        }
    }

    problems
}

#[test]
fn the_tracked_tree_matches_its_baseline() {
    let root = repository_root();
    let measured = measure_tracked_files(&root);
    let baseline_text = std::fs::read_to_string(baseline_path())
        .expect("files_stay_small.baseline deberia existir");
    let baseline = parse_baseline(&baseline_text);

    let problems = baseline_problems(&measured, &baseline);
    assert!(
        problems.is_empty(),
        "el tamaño de los ficheros no coincide con files_stay_small.baseline:\n  {}\n\n{WHAT_TO_DO}",
        problems.join("\n  ")
    );
}

#[test]
fn the_baseline_has_entries_so_the_guard_has_work() {
    let baseline_text = std::fs::read_to_string(baseline_path())
        .expect("files_stay_small.baseline deberia existir");
    let baseline = parse_baseline(&baseline_text);

    assert!(!baseline.is_empty());
}

#[test]
fn a_file_missing_from_the_baseline_above_its_threshold_is_caught() {
    let measured = BTreeMap::from([("rfirma-app/src/new.ts".to_owned(), 501)]);
    let baseline = BTreeMap::new();

    let problems = baseline_problems(&measured, &baseline);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("new.ts"));
    assert!(problems[0].contains("no está en el baseline"));
}

#[test]
fn a_file_missing_from_the_baseline_under_its_threshold_is_not_reported() {
    let measured = BTreeMap::from([("rfirma-app/src/small.ts".to_owned(), 10)]);
    let baseline = BTreeMap::new();

    assert!(baseline_problems(&measured, &baseline).is_empty());
}

#[test]
fn a_listed_file_that_grew_is_caught() {
    let measured = BTreeMap::from([("rfirma-app/src-tauri/src/big.rs".to_owned(), 520)]);
    let baseline = BTreeMap::from([("rfirma-app/src-tauri/src/big.rs".to_owned(), 501)]);

    let problems = baseline_problems(&measured, &baseline);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("creció"));
}

#[test]
fn a_listed_file_that_shrank_without_updating_the_baseline_is_caught() {
    let measured = BTreeMap::from([("rfirma-app/src-tauri/src/big.rs".to_owned(), 510)]);
    let baseline = BTreeMap::from([("rfirma-app/src-tauri/src/big.rs".to_owned(), 520)]);

    let problems = baseline_problems(&measured, &baseline);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("bajó"));
}

#[test]
fn a_listed_file_that_dropped_below_its_threshold_is_caught() {
    let measured = BTreeMap::from([("rfirma-app/src-tauri/src/shrunk.rs".to_owned(), 450)]);
    let baseline = BTreeMap::from([("rfirma-app/src-tauri/src/shrunk.rs".to_owned(), 520)]);

    let problems = baseline_problems(&measured, &baseline);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("sale del baseline"));
}

#[test]
fn a_listed_file_that_no_longer_exists_is_caught() {
    let measured = BTreeMap::new();
    let baseline = BTreeMap::from([("rfirma-app/src-tauri/src/gone.rs".to_owned(), 520)]);

    let problems = baseline_problems(&measured, &baseline);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("ya no existe"));
}

#[test]
fn a_listed_file_that_matches_exactly_reports_nothing() {
    let measured = BTreeMap::from([("rfirma-app/src-tauri/src/stable.rs".to_owned(), 520)]);
    let baseline = BTreeMap::from([("rfirma-app/src-tauri/src/stable.rs".to_owned(), 520)]);

    assert!(baseline_problems(&measured, &baseline).is_empty());
}

#[test]
fn a_directory_named_tests_marks_every_file_under_it_as_a_test() {
    assert!(is_a_test_file("rfirma-app/src-tauri/tests/native_cycle.rs"));
    assert!(is_a_test_file(
        "rfirma-app/src-tauri/src/site/adapters/relay/tests/support.rs"
    ));
}

#[test]
fn a_module_sibling_called_tests_rs_is_a_test_without_a_tests_directory() {
    assert!(is_a_test_file(
        "rfirma-app/src-tauri/src/site/application/errand/tests.rs"
    ));
    assert!(!is_a_test_file(
        "rfirma-app/src-tauri/src/site/application/errand/desk.rs"
    ));
}

#[test]
fn a_dot_test_ts_or_tsx_suffix_is_a_test() {
    assert!(is_a_test_file("rfirma-app/src/App.test.tsx"));
    assert!(is_a_test_file("rfirma-app/src/sede/siteErrands.test.ts"));
    assert!(!is_a_test_file("rfirma-app/src/App.tsx"));
}

#[test]
fn the_i18n_locales_prefix_is_exempt() {
    assert!(is_exempt("rfirma-app/src/i18n/locales/es.ts"));
    assert!(!is_exempt("rfirma-app/src/i18n/i18n.ts"));
}

#[test]
fn the_baseline_parses_path_and_line_count() {
    let baseline =
        parse_baseline("rfirma-app/src-tauri/src/a.rs 520\nrfirma-app/src/App.tsx 700\n");

    assert_eq!(
        baseline,
        BTreeMap::from([
            ("rfirma-app/src-tauri/src/a.rs".to_owned(), 520),
            ("rfirma-app/src/App.tsx".to_owned(), 700),
        ])
    );
}

#[test]
fn blank_lines_do_not_count() {
    assert_eq!(non_blank_lines("a\n\n  \nb\n"), 2);
}
