//! Guarda de grada A: el catálogo de la suite de conformidad está completo, su vocabulario es el
//! cerrado, cada entrada tiene cuerpo en el código, la referencia cuadra con ambos y el corpus del
//! anexo A1 sigue decidido.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Una entrada del catálogo, leída sin las estructuras del ejemplo: la guarda vigila la forma, así
/// que no puede compartir el parser que la da por buena.
#[derive(Debug, Clone, Default)]
struct Entry {
    id: String,
    suite: String,
    chapter: String,
    citation: String,
    statement: String,
    driven: bool,
    harness: Option<String>,
    needs: Vec<String>,
    question: Option<String>,
    warning: Option<String>,
    unmeasurable: Option<String>,
    greeting: bool,
    cited_cards: BTreeSet<String>,
}

/// Un resultado previsto de la referencia, leído igual de crudo que las entradas del catálogo.
#[derive(Debug, Clone)]
struct Known {
    id: String,
    outcome: String,
    cause: String,
}

/// Una ficha del anexo A1 y lo que el anexo decide sobre ella.
#[derive(Debug, Clone, PartialEq, Eq)]
struct A1Card {
    id: String,
    unobservable: Option<String>,
}

const THE_KNOWN_OUTCOMES: [&str; 2] = ["conforme", "no-conforme"];

const THE_SUITES: [&str; 10] = [
    "saludo",
    "transporte.websocket",
    "transporte.service",
    "versiones",
    "operaciones",
    "operaciones.firma",
    "operaciones.disco",
    "operaciones.lote",
    "errores",
    "parametros",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn catalogue_dir() -> PathBuf {
    repository_root().join("rfirma-app/src-tauri/examples/conformance/catalogue")
}

/// Los ficheros del catálogo repartido, uno por conjunto, en un orden estable e independiente del
/// sistema de ficheros.
fn catalogue_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(catalogue_dir())
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", catalogue_dir().display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect();
    files.sort();
    files
}

fn reference_dir() -> PathBuf {
    repository_root().join("rfirma-app/src-tauri/examples/conformance/reference")
}

fn reference_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(reference_dir())
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", reference_dir().display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect();
    files.sort();
    files
}

fn checks_path() -> PathBuf {
    repository_root().join("rfirma-app/src-tauri/examples/conformance/checks.rs")
}

fn errors_chapter_path() -> PathBuf {
    repository_root().join("docs/afirma/1.9.2/15-errores.md")
}

fn annex_path() -> PathBuf {
    repository_root().join("docs/afirma/1.9.2/A1-bugs-autofirma.md")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", path.display()))
}

fn entries_in(catalogue: &str) -> Vec<Entry> {
    let parsed: toml::Table = toml::from_str(catalogue)
        .unwrap_or_else(|error| panic!("el catalogo no es TOML valido: {error}"));
    parsed
        .get("check")
        .and_then(toml::Value::as_array)
        .expect("el catalogo deberia tener entradas [[check]]")
        .iter()
        .map(entry_of)
        .collect()
}

fn entry_of(value: &toml::Value) -> Entry {
    let string = |key: &str| {
        value
            .get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let optional = |key: &str| {
        value
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    Entry {
        id: string("id"),
        suite: string("suite"),
        chapter: string("chapter"),
        citation: string("citation"),
        statement: string("statement"),
        driven: value.get("drive").is_some(),
        harness: optional("harness"),
        needs: value
            .get("needs")
            .and_then(toml::Value::as_array)
            .map(|needs| {
                needs
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        question: optional("question"),
        warning: optional("warning"),
        unmeasurable: optional("unmeasurable"),
        greeting: value
            .get("greeting")
            .and_then(toml::Value::as_bool)
            .unwrap_or_default(),
        cited_cards: cards_cited_in(&value.to_string()),
    }
}

fn known_in(reference: &str) -> Vec<Known> {
    let parsed: toml::Table = toml::from_str(reference)
        .unwrap_or_else(|error| panic!("la referencia no es TOML valido: {error}"));
    let string = |value: &toml::Value, key: &str| {
        value
            .get(key)
            .and_then(toml::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    parsed
        .get("known")
        .and_then(toml::Value::as_array)
        .map(|known| {
            known
                .iter()
                .map(|value| Known {
                    id: string(value, "id"),
                    outcome: string(value, "outcome"),
                    cause: string(value, "cause"),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Las fichas `BUG-NN` citadas en un texto, por donde quiera que la entrada las cite.
fn cards_cited_in(text: &str) -> BTreeSet<String> {
    let mut cited = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find("BUG-") {
        let after = &rest[at + 4..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        let starts_a_word = at == 0 || !rest[..at].ends_with(|c: char| c.is_alphanumeric());
        if starts_a_word && digits.len() >= 2 {
            cited.insert(format!("BUG-{digits}"));
        }
        rest = after;
    }
    cited
}

/// Los códigos de la tabla sinóptica del capítulo 15, por las filas que abren con uno.
fn saf_codes_in_the_table(chapter: &str) -> BTreeSet<String> {
    chapter
        .lines()
        .filter_map(|line| line.strip_prefix("| `SAF_"))
        .filter_map(|rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (digits.len() == 2).then(|| format!("SAF_{digits}"))
        })
        .collect()
}

/// Los códigos `SAF_NN` que nombra un texto, por donde quiera que los nombre.
fn saf_codes_named_in(text: &str) -> BTreeSet<String> {
    text.match_indices("SAF_")
        .filter_map(|(at, _)| {
            let digits: String = text[at + 4..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            (digits.len() == 2).then(|| format!("SAF_{digits}"))
        })
        .collect()
}

fn codes_of_the_table_without_an_entry(
    table: &BTreeSet<String>,
    named: &BTreeSet<String>,
) -> Vec<String> {
    table.difference(named).cloned().collect()
}

/// Los nombres de arnés que el código sabe correr, leídos de su lista como texto.
fn harnesses_in(checks: &str) -> BTreeSet<String> {
    let from = checks
        .find("THE_HARNESSES")
        .expect("checks.rs deberia declarar THE_HARNESSES");
    let list = &checks[from..];
    let end = list
        .find("];")
        .expect("la lista de arneses deberia cerrarse");
    list[..end]
        .match_indices('"')
        .map(|(at, _)| at)
        .collect::<Vec<_>>()
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .map(|pair| list[pair[0] + 1..pair[1]].to_owned())
        .collect()
}

/// Los capítulos con fichero en el manual, por el prefijo numérico de su nombre.
fn chapters_in(manual: &Path) -> BTreeSet<String> {
    std::fs::read_dir(manual)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", manual.display()))
        .filter_map(|entry| entry.ok()?.file_name().to_str().map(str::to_owned))
        .filter(|name| name.ends_with(".md"))
        .filter_map(|name| name.split('-').next().map(str::to_owned))
        .filter(|prefix| prefix.len() == 2 && prefix.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

fn a1_cards_in(annex: &str) -> Vec<A1Card> {
    let mut cards: Vec<A1Card> = Vec::new();
    for line in annex.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("### BUG-") {
            if let Some(colon) = rest.find(':') {
                cards.push(A1Card {
                    id: format!("BUG-{}", rest[..colon].trim()),
                    unobservable: None,
                });
            }
            continue;
        }
        if let (Some(card), Some(at)) = (cards.last_mut(), trimmed.find("**No observable:**")) {
            card.unobservable = Some(trimmed[at + "**No observable:**".len()..].trim().to_owned());
        }
    }
    cards
}

fn duplicate_ids(entries: &[Entry]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .filter(|entry| !seen.insert(entry.id.clone()))
        .map(|entry| entry.id.clone())
        .collect()
}

fn entries_without_a_body(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !entry.driven && entry.unmeasurable.is_none())
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

fn entries_with_an_unknown_suite(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !THE_SUITES.contains(&entry.suite.as_str()))
        .map(|entry| format!("{}: {}", entry.id, entry.suite))
        .collect()
}

fn entries_whose_chapter_has_no_file(
    entries: &[Entry],
    chapters: &BTreeSet<String>,
) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !chapters.contains(&entry.chapter))
        .map(|entry| format!("{}: {}", entry.id, entry.chapter))
        .collect()
}

/// Los saludos que no son la primera entrada de su conjunto, o que no se conducen de verdad.
fn greetings_that_do_not_open_their_suite(entries: &[Entry]) -> Vec<String> {
    let mut opened = BTreeSet::new();
    entries
        .iter()
        .filter_map(|entry| {
            let first_of_its_suite = opened.insert(entry.suite.clone());
            match entry.greeting {
                true if !first_of_its_suite => Some(format!("{}: no abre su conjunto", entry.id)),
                true if !entry.driven => Some(format!("{}: saludo sin conducir", entry.id)),
                _ => None,
            }
        })
        .collect()
}

fn entries_with_an_empty_field(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.statement.trim().is_empty() || entry.citation.trim().is_empty())
        .map(|entry| entry.id.clone())
        .collect()
}

fn unmeasurable_entries_that_are_wrong(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| match entry.unmeasurable.as_deref() {
            Some(motive) if motive.trim().is_empty() => Some(format!("{}: sin motivo", entry.id)),
            Some(_) if entry.driven => Some(format!("{}: no medible pero conducida", entry.id)),
            _ => None,
        })
        .collect()
}

fn person_entries_without_a_question_or_a_warning(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.needs.iter().any(|need| need == "persona"))
        .flat_map(|entry| {
            [("pregunta", &entry.question), ("aviso", &entry.warning)]
                .into_iter()
                .filter(|(_, said)| {
                    said.as_deref()
                        .map(str::trim)
                        .unwrap_or_default()
                        .is_empty()
                })
                .map(|(what, _)| format!("{}: sin {what}", entry.id))
        })
        .collect()
}

fn cited_cards_that_do_not_exist(entries: &[Entry], cards: &BTreeSet<String>) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| {
            entry
                .cited_cards
                .iter()
                .filter(|card| !cards.contains(*card))
                .map(move |card| format!("{}: {card}", entry.id))
        })
        .collect()
}

fn undecided_cards(cards: &[A1Card], cited: &BTreeSet<String>) -> Vec<String> {
    cards
        .iter()
        .filter(|card| !cited.contains(&card.id) && card.unobservable.is_none())
        .map(|card| card.id.clone())
        .collect()
}

fn unmotivated_cards(cards: &[A1Card]) -> Vec<String> {
    cards
        .iter()
        .filter(|card| {
            card.unobservable
                .as_deref()
                .is_some_and(|motive| motive.trim().is_empty())
        })
        .map(|card| card.id.clone())
        .collect()
}

/// Lo que la referencia prevé de una comprobación que el catálogo no tiene, con un resultado fuera
/// de su vocabulario o con una causa que no es ficha del anexo A1.
fn known_results_that_are_wrong(
    known: &[Known],
    entries: &[Entry],
    cards: &BTreeSet<String>,
) -> Vec<String> {
    known
        .iter()
        .flat_map(|known| {
            let mut wrong = Vec::new();
            if !entries.iter().any(|entry| entry.id == known.id) {
                wrong.push(format!("{}: no está en el catálogo", known.id));
            }
            if !THE_KNOWN_OUTCOMES.contains(&known.outcome.as_str()) {
                wrong.push(format!(
                    "{}: «{}» no es conforme ni no-conforme",
                    known.id, known.outcome
                ));
            }
            if !cards.contains(&known.cause) {
                wrong.push(format!("{}: {} no es ficha de A1", known.id, known.cause));
            }
            wrong
        })
        .collect()
}

fn the_references() -> Vec<String> {
    reference_files().iter().map(|path| read(path)).collect()
}

fn the_catalogue() -> Vec<Entry> {
    catalogue_files()
        .iter()
        .flat_map(|path| entries_in(&read(path)))
        .collect()
}

#[test]
fn every_check_has_a_unique_identifier_and_a_body_in_the_code() {
    let entries = the_catalogue();
    assert!(
        entries.len() >= 34,
        "el catalogo deberia tener las treinta y cuatro exigencias; hay {}",
        entries.len()
    );
    let harnesses = harnesses_in(&read(&checks_path()));

    assert!(
        duplicate_ids(&entries).is_empty(),
        "hay identificadores repetidos en el catalogo:\n  {}",
        duplicate_ids(&entries).join("\n  ")
    );
    assert!(
        entries_without_a_body(&entries).is_empty(),
        "hay entradas que ni se conducen ni se declaran no medibles:\n  {}",
        entries_without_a_body(&entries).join("\n  ")
    );
    assert!(
        harnesses_named_without_a_body(&entries, &harnesses).is_empty(),
        "hay entradas que piden un arnes que el codigo no tiene:\n  {}",
        harnesses_named_without_a_body(&entries, &harnesses).join("\n  ")
    );
    assert!(
        harnesses_left_out_of_the_catalogue(&entries, &harnesses).is_empty(),
        "hay arneses en el codigo que ninguna entrada del catalogo nombra:\n  {}",
        harnesses_left_out_of_the_catalogue(&entries, &harnesses).join("\n  ")
    );
}

#[test]
fn every_check_declares_a_suite_of_the_vocabulary_and_a_chapter_of_the_manual() {
    let entries = the_catalogue();
    let chapters = chapters_in(&repository_root().join("docs/afirma/1.9.2"));

    assert!(
        entries_with_an_unknown_suite(&entries).is_empty(),
        "hay entradas con un conjunto fuera del vocabulario:\n  {}",
        entries_with_an_unknown_suite(&entries).join("\n  ")
    );
    assert!(
        entries_whose_chapter_has_no_file(&entries, &chapters).is_empty(),
        "hay entradas que citan un capitulo sin fichero en docs/afirma/1.9.2/:\n  {}",
        entries_whose_chapter_has_no_file(&entries, &chapters).join("\n  ")
    );
}

#[test]
fn every_check_says_what_the_protocol_demands_and_where_it_is_written() {
    let entries = the_catalogue();

    assert!(
        entries_with_an_empty_field(&entries).is_empty(),
        "hay entradas sin enunciado o sin cita:\n  {}",
        entries_with_an_empty_field(&entries).join("\n  ")
    );
    assert!(
        unmeasurable_entries_that_are_wrong(&entries).is_empty(),
        "hay entradas no medibles mal declaradas:\n  {}",
        unmeasurable_entries_that_are_wrong(&entries).join("\n  ")
    );
    assert!(
        person_entries_without_a_question_or_a_warning(&entries).is_empty(),
        "hay entradas que necesitan a una persona y no traen su pregunta o su aviso:\n  {}",
        person_entries_without_a_question_or_a_warning(&entries).join("\n  ")
    );
}

#[test]
fn every_a1_card_is_decided_and_no_check_cites_one_that_does_not_exist() {
    let entries = the_catalogue();
    let cards = a1_cards_in(&read(&annex_path()));
    assert!(
        cards.len() >= 26,
        "el anexo A1 deberia tener al menos 26 fichas; encontradas {}",
        cards.len()
    );
    let existing: BTreeSet<String> = cards.iter().map(|card| card.id.clone()).collect();
    let cited: BTreeSet<String> = entries
        .iter()
        .flat_map(|entry| entry.cited_cards.iter().cloned())
        .chain(the_references().iter().flat_map(|raw| cards_cited_in(raw)))
        .collect();

    assert!(
        cited_cards_that_do_not_exist(&entries, &existing).is_empty(),
        "hay entradas que citan fichas inexistentes de A1:\n  {}",
        cited_cards_that_do_not_exist(&entries, &existing).join("\n  ")
    );
    assert!(
        undecided_cards(&cards, &cited).is_empty(),
        "hay fichas de A1 sin citar desde el catalogo y sin marca de no observable:\n  {}",
        undecided_cards(&cards, &cited).join("\n  ")
    );
    assert!(
        unmotivated_cards(&cards).is_empty(),
        "hay fichas de A1 marcadas como no observables sin motivo:\n  {}",
        unmotivated_cards(&cards).join("\n  ")
    );
}

#[test]
fn every_code_of_the_error_table_is_closed_against_the_catalogue() {
    let table = saf_codes_in_the_table(&read(&errors_chapter_path()));
    let named: BTreeSet<String> = catalogue_files()
        .iter()
        .flat_map(|path| saf_codes_named_in(&read(path)))
        .collect();

    assert_eq!(
        table.len(),
        53,
        "la tabla del capitulo 15 deberia tener los 53 codigos"
    );
    assert!(
        codes_of_the_table_without_an_entry(&table, &named).is_empty(),
        "hay codigos de la tabla del capitulo 15 sin entrada, sin familia y sin declararse no \
         medibles:\n  {}",
        codes_of_the_table_without_an_entry(&table, &named).join("\n  ")
    );
}

#[test]
fn a_code_of_the_table_the_catalogue_never_names_is_caught_and_named() {
    let table = saf_codes_in_the_table(
        "| `SAF_00` | `ERROR_CANNOT_READ_DATA` |\n| `SAF_07` | `ERROR_CANNOT_FIND_KEYSTORE` |\n",
    );
    let named = saf_codes_named_in("expects_saf = \"SAF_00\"\nstatement = \"SAF_070 no cuenta.\"");

    assert_eq!(
        codes_of_the_table_without_an_entry(&table, &named),
        vec!["SAF_07"]
    );
}

#[test]
fn every_known_result_of_the_reference_names_a_check_of_the_catalogue_and_a_card_of_a1() {
    let entries = the_catalogue();
    let cards: BTreeSet<String> = a1_cards_in(&read(&annex_path()))
        .into_iter()
        .map(|card| card.id)
        .collect();
    let known: Vec<Known> = the_references()
        .iter()
        .flat_map(|raw| known_in(raw))
        .collect();

    assert!(
        !known.is_empty(),
        "deberia haber al menos una referencia con resultados previstos"
    );
    assert!(
        known_results_that_are_wrong(&known, &entries, &cards).is_empty(),
        "hay resultados previstos de la referencia que no cuadran:\n  {}",
        known_results_that_are_wrong(&known, &entries, &cards).join("\n  ")
    );
}

#[test]
fn a_known_result_outside_the_catalogue_the_vocabulary_or_the_annex_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n",
    );
    let known = known_in(
        "[[known]]\nid = \"a_one\"\noutcome = \"no-observable\"\ncause = \"BUG-01\"\nnote = \"x\"\n\n\
         [[known]]\nid = \"a_two\"\noutcome = \"no-conforme\"\ncause = \"ADR-0005\"\nnote = \"y\"\n",
    );
    let cards: BTreeSet<String> = ["BUG-01"].map(str::to_owned).into();

    assert_eq!(
        known_results_that_are_wrong(&known, &entries, &cards),
        vec![
            "a_one: «no-observable» no es conforme ni no-conforme",
            "a_two: no está en el catálogo",
            "a_two: ADR-0005 no es ficha de A1",
        ]
    );
}

#[test]
fn a_repeated_identifier_is_caught_and_named() {
    let entries = entries_in(
        r#"
[[check]]
id = "a_one"
drive = { mode = "v4", script = "selectcert" }

[[check]]
id = "a_one"
drive = { mode = "v4", script = "selectcert" }
"#,
    );
    assert_eq!(duplicate_ids(&entries), vec!["a_one"]);
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
fn a_suite_outside_the_vocabulary_and_a_chapter_without_a_file_are_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\nsuite = \"inventado\"\nchapter = \"99\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n",
    );
    let chapters: BTreeSet<String> = ["05"].map(str::to_owned).into();

    assert_eq!(
        entries_with_an_unknown_suite(&entries),
        vec!["a_one: inventado"]
    );
    assert_eq!(
        entries_whose_chapter_has_no_file(&entries, &chapters),
        vec!["a_one: 99"]
    );
}

#[test]
fn every_greeting_opens_its_suite_and_the_socket_suite_opens_with_one() {
    let entries = the_catalogue();

    assert!(
        greetings_that_do_not_open_their_suite(&entries).is_empty(),
        "hay saludos que no abren su conjunto:\n  {}",
        greetings_that_do_not_open_their_suite(&entries).join("\n  ")
    );
    assert!(
        entries
            .iter()
            .find(|entry| entry.suite == "transporte.service")
            .is_some_and(|entry| entry.greeting),
        "el conjunto transporte.service deberia abrir con la comprobacion de saludo de su carril"
    );
}

#[test]
fn a_greeting_behind_another_check_or_without_a_drive_is_caught_and_named() {
    let entries = entries_in(
        r#"
[[check]]
id = "a_one"
suite = "transporte.service"
drive = { mode = "service", script = "selectcert" }

[[check]]
id = "a_late_greeting"
suite = "transporte.service"
drive = { mode = "service", script = "selectcert" }
greeting = true

[[check]]
id = "an_undriven_greeting"
suite = "errores"
unmeasurable = "Nada que conducir."
greeting = true
"#,
    );

    assert_eq!(
        greetings_that_do_not_open_their_suite(&entries),
        vec![
            "a_late_greeting: no abre su conjunto",
            "an_undriven_greeting: saludo sin conducir"
        ]
    );
}

#[test]
fn an_empty_statement_or_citation_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\nstatement = \"  \"\ncitation = \"A.java:1\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n",
    );
    assert_eq!(entries_with_an_empty_field(&entries), vec!["a_one"]);
}

#[test]
fn an_unmeasurable_check_that_is_driven_or_unmotivated_is_caught_and_named() {
    let driven = entries_in(
        "[[check]]\nid = \"a_one\"\nunmeasurable = \"no llega al cable\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n",
    );
    assert_eq!(
        unmeasurable_entries_that_are_wrong(&driven),
        vec!["a_one: no medible pero conducida"]
    );

    let unmotivated = entries_in("[[check]]\nid = \"a_two\"\nunmeasurable = \"\"\n");
    assert_eq!(
        unmeasurable_entries_that_are_wrong(&unmotivated),
        vec!["a_two: sin motivo"]
    );
}

#[test]
fn a_check_that_needs_a_person_without_a_question_or_a_warning_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\nneeds = [\"persona\"]\ndrive = { mode = \"v4\", script = \"save\" }\n",
    );
    assert_eq!(
        person_entries_without_a_question_or_a_warning(&entries),
        vec!["a_one: sin pregunta", "a_one: sin aviso"]
    );

    let answered = entries_in(
        "[[check]]\nid = \"a_two\"\nneeds = [\"persona\"]\nquestion = \"¿se pidió destino? [s/n]\"\nwarning = \"Va a aparecer la ventana de destino.\"\ndrive = { mode = \"v4\", script = \"save\" }\n",
    );
    assert!(person_entries_without_a_question_or_a_warning(&answered).is_empty());
}

#[test]
fn a_cited_card_that_does_not_exist_and_an_undecided_one_are_both_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ncause = \"BUG-99\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n",
    );
    let annex = "### BUG-01: Uno\n* **No observable:** no llega al cable.\n\n### BUG-02: Dos\n";
    let cards = a1_cards_in(annex);
    let existing: BTreeSet<String> = cards.iter().map(|card| card.id.clone()).collect();
    let cited: BTreeSet<String> = entries
        .iter()
        .flat_map(|entry| entry.cited_cards.iter().cloned())
        .collect();

    assert_eq!(
        cited_cards_that_do_not_exist(&entries, &existing),
        vec!["a_one: BUG-99"]
    );
    assert_eq!(undecided_cards(&cards, &cited), vec!["BUG-02"]);
}

#[test]
fn a_card_marked_unobservable_without_a_reason_is_caught_and_named() {
    let cards = a1_cards_in("### BUG-03: Tres\n* **No observable:**\n");
    assert_eq!(unmotivated_cards(&cards), vec!["BUG-03"]);
}

#[test]
fn the_reader_of_harnesses_takes_the_names_of_the_list_and_nothing_else() {
    let checks = "const OTHER: &str = \"no\";\npub(crate) const THE_HARNESSES: &[&str] = &[\n    \"a_one\",\n    \"a_two\",\n];\nfn later() -> &'static str { \"tampoco\" }";
    assert_eq!(
        harnesses_in(checks),
        ["a_one", "a_two"].map(str::to_owned).into()
    );
}
