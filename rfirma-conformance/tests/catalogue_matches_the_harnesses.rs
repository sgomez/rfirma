//! El catálogo y los arneses se cruzan: cada entrada tiene cuerpo y cada arnés tiene entrada.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct Entry {
    id: String,
    driven: bool,
    unmeasurable: bool,
    harness: Option<String>,
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", path.display()))
}

fn entries_in(catalogue: &str) -> Vec<Entry> {
    let parsed: toml::Table = toml::from_str(catalogue)
        .unwrap_or_else(|error| panic!("el catálogo no es TOML válido: {error}"));
    parsed
        .get("check")
        .and_then(toml::Value::as_array)
        .expect("el catálogo debería tener entradas [[check]]")
        .iter()
        .map(|value| Entry {
            id: value
                .get("id")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            driven: value.get("drive").is_some(),
            unmeasurable: value.get("unmeasurable").is_some(),
            harness: value
                .get("harness")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
        })
        .collect()
}

fn the_catalogue() -> Vec<Entry> {
    let dir = crate_dir().join("catalogue");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|path| entries_in(&read(path)))
        .collect()
}

/// Los nombres de arnés que el código sabe correr, leídos de su lista como texto.
fn harnesses_in(checks: &str) -> BTreeSet<String> {
    let from = checks
        .find("THE_HARNESSES")
        .expect("checks.rs debería declarar THE_HARNESSES");
    let list = &checks[from..];
    let end = list
        .find("];")
        .expect("la lista de arneses debería cerrarse");
    list[..end]
        .match_indices('"')
        .map(|(at, _)| at)
        .collect::<Vec<_>>()
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .map(|pair| list[pair[0] + 1..pair[1]].to_owned())
        .collect()
}

fn entries_without_a_body(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !entry.driven && !entry.unmeasurable)
        .map(|entry| entry.id.clone())
        .collect()
}

fn harnesses_named_without_a_body(entries: &[Entry], known: &BTreeSet<String>) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| {
            let harness = entry.harness.as_ref()?;
            (!known.contains(harness)).then(|| format!("{}: {harness}", entry.id))
        })
        .collect()
}

fn harnesses_left_out_of_the_catalogue(entries: &[Entry], known: &BTreeSet<String>) -> Vec<String> {
    let named: BTreeSet<&String> = entries
        .iter()
        .filter_map(|entry| entry.harness.as_ref())
        .collect();
    known
        .iter()
        .filter(|harness| !named.contains(harness))
        .cloned()
        .collect()
}

#[test]
fn every_check_has_a_body_in_the_code_and_every_harness_a_check() {
    let entries = the_catalogue();
    let harnesses = harnesses_in(&read(&crate_dir().join("src/checks.rs")));

    assert!(
        entries_without_a_body(&entries).is_empty(),
        "hay entradas que ni se conducen ni se declaran no medibles:\n  {}",
        entries_without_a_body(&entries).join("\n  ")
    );
    assert!(
        harnesses_named_without_a_body(&entries, &harnesses).is_empty(),
        "hay entradas que piden un arnés que el código no tiene:\n  {}",
        harnesses_named_without_a_body(&entries, &harnesses).join("\n  ")
    );
    assert!(
        harnesses_left_out_of_the_catalogue(&entries, &harnesses).is_empty(),
        "hay arneses en el código que ninguna entrada del catálogo nombra:\n  {}",
        harnesses_left_out_of_the_catalogue(&entries, &harnesses).join("\n  ")
    );
}

#[test]
fn an_entry_without_a_body_is_caught_and_named() {
    let entries = entries_in("[[check]]\nid = \"a_one\"\n");
    assert_eq!(entries_without_a_body(&entries), vec!["a_one"]);
}

#[test]
fn a_harness_that_nobody_wrote_and_one_that_nobody_calls_are_both_caught() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\nharness = \"an_absent_one\"\n",
    );
    let known: BTreeSet<String> = ["a_lonely_one"].map(str::to_owned).into();

    assert_eq!(
        harnesses_named_without_a_body(&entries, &known),
        vec!["a_one: an_absent_one"]
    );
    assert_eq!(
        harnesses_left_out_of_the_catalogue(&entries, &known),
        vec!["a_lonely_one"]
    );
}

#[test]
fn the_reader_of_harnesses_takes_the_names_of_the_list_and_nothing_else() {
    let checks = "const OTHER: &str = \"no\";\npub(crate) const THE_HARNESSES: &[&str] = &[\n    \"a_one\",\n    \"a_two\",\n];\nfn later() -> &'static str { \"tampoco\" }";
    assert_eq!(
        harnesses_in(checks),
        ["a_one", "a_two"].map(str::to_owned).into()
    );
}
