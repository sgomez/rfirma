//! Guarda de grada A: el catálogo de la suite de conformidad está completo, su vocabulario es el
//! cerrado, cada entrada tiene cuerpo en el código y el corpus del anexo A1 sigue decidido.

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
    unmeasurable: Option<String>,
    cited_cards: BTreeSet<String>,
    expectations: Vec<Expectation>,
}

/// Lo que la línea base declara de un perfil, leído igual de crudo que el resto de la entrada.
#[derive(Debug, Clone)]
struct Expectation {
    profile: String,
    verdict: String,
    cause: Option<String>,
}

/// Una ficha del anexo A1 y lo que el anexo decide sobre ella.
#[derive(Debug, Clone, PartialEq, Eq)]
struct A1Card {
    id: String,
    unobservable: Option<String>,
}

const THE_PROFILES: [&str; 2] = ["autofirma", "rfirma"];

const THE_VERDICTS: [&str; 3] = ["conforme", "no-conforme", "no-observable"];

const THE_SUITES: [&str; 6] = [
    "saludo",
    "transporte.websocket",
    "transporte.service",
    "versiones",
    "operaciones",
    "errores",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn catalogue_path() -> PathBuf {
    repository_root().join("rfirma-app/src-tauri/examples/conformance/catalogue.toml")
}

fn checks_path() -> PathBuf {
    repository_root().join("rfirma-app/src-tauri/examples/conformance/checks.rs")
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
        unmeasurable: optional("unmeasurable"),
        cited_cards: cards_cited_in(&value.to_string()),
        expectations: expectations_of(value),
    }
}

fn expectations_of(value: &toml::Value) -> Vec<Expectation> {
    let Some(declared) = value.get("expect").and_then(toml::Value::as_table) else {
        return Vec::new();
    };
    declared
        .iter()
        .map(|(profile, expectation)| Expectation {
            profile: profile.clone(),
            verdict: expectation
                .get("verdict")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            cause: expectation
                .get("cause")
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
        })
        .collect()
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

fn person_entries_without_a_question(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| {
            entry.needs.iter().any(|need| need == "persona")
                && entry
                    .question
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty()
        })
        .map(|entry| entry.id.clone())
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

/// Las entradas que no declaran expectativa para algún perfil conocido, o la declaran con un
/// veredicto que no existe.
fn entries_whose_baseline_is_incomplete(entries: &[Entry]) -> Vec<String> {
    let mut wrong = Vec::new();
    for entry in entries {
        for profile in THE_PROFILES {
            match entry
                .expectations
                .iter()
                .find(|expectation| expectation.profile == profile)
            {
                None => wrong.push(format!("{}: sin expectativa para {profile}", entry.id)),
                Some(expectation) if !THE_VERDICTS.contains(&expectation.verdict.as_str()) => {
                    wrong.push(format!(
                        "{}: {profile} espera «{}», que no es un veredicto",
                        entry.id, expectation.verdict
                    ));
                }
                Some(_) => {}
            }
        }
    }
    wrong
}

/// Una expectativa distinta de `conforme` sin causa: no se declara un incumplimiento porque sí.
fn expectations_that_deviate_without_a_cause(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| {
            entry
                .expectations
                .iter()
                .filter(|expectation| {
                    expectation.verdict != "conforme" && expectation.cause.is_none()
                })
                .map(move |expectation| format!("{}: {}", entry.id, expectation.profile))
        })
        .collect()
}

/// Las causas que no resuelven ni a ficha del anexo A1 ni a fichero de ADR.
fn causes_that_do_not_resolve(
    entries: &[Entry],
    cards: &BTreeSet<String>,
    adrs: &BTreeSet<String>,
) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| {
            entry.expectations.iter().filter_map(move |expectation| {
                let cause = expectation.cause.as_deref()?;
                let resolves = cards.contains(cause) || adrs.contains(cause);
                (!resolves).then(|| format!("{}: {cause}", entry.id))
            })
        })
        .collect()
}

/// Los ADR con fichero en `docs/adr/`, por su identificador `ADR-NNNN`.
fn adrs_in(folder: &Path) -> BTreeSet<String> {
    std::fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", folder.display()))
        .filter_map(|entry| entry.ok()?.file_name().to_str().map(str::to_owned))
        .filter(|name| name.ends_with(".md"))
        .filter_map(|name| name.split('-').next().map(str::to_owned))
        .filter(|number| number.len() == 4 && number.chars().all(|c| c.is_ascii_digit()))
        .map(|number| format!("ADR-{number}"))
        .collect()
}

fn the_catalogue() -> Vec<Entry> {
    entries_in(&read(&catalogue_path()))
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
        person_entries_without_a_question(&entries).is_empty(),
        "hay entradas que necesitan a una persona y no traen su pregunta:\n  {}",
        person_entries_without_a_question(&entries).join("\n  ")
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
fn every_check_declares_what_the_baseline_expects_of_every_profile() {
    let entries = the_catalogue();
    let cards: BTreeSet<String> = a1_cards_in(&read(&annex_path()))
        .into_iter()
        .map(|card| card.id)
        .collect();
    let adrs = adrs_in(&repository_root().join("docs/adr"));

    assert!(
        entries_whose_baseline_is_incomplete(&entries).is_empty(),
        "hay entradas cuya linea base no cubre todos los perfiles:\n  {}",
        entries_whose_baseline_is_incomplete(&entries).join("\n  ")
    );
    assert!(
        expectations_that_deviate_without_a_cause(&entries).is_empty(),
        "hay expectativas distintas de conforme sin causa que las explique:\n  {}",
        expectations_that_deviate_without_a_cause(&entries).join("\n  ")
    );
    assert!(
        causes_that_do_not_resolve(&entries, &cards, &adrs).is_empty(),
        "hay causas que no son ni ficha de A1 ni fichero de ADR:\n  {}",
        causes_that_do_not_resolve(&entries, &cards, &adrs).join("\n  ")
    );
}

#[test]
fn a_baseline_without_a_profile_or_with_an_invented_verdict_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n\n[check.expect.autofirma]\nverdict = \"regular\"\n",
    );

    assert_eq!(
        entries_whose_baseline_is_incomplete(&entries),
        vec![
            "a_one: autofirma espera «regular», que no es un veredicto",
            "a_one: sin expectativa para rfirma",
        ]
    );
}

#[test]
fn an_expectation_that_deviates_without_a_cause_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n\n[check.expect.autofirma]\nverdict = \"no-conforme\"\n\n[check.expect.rfirma]\nverdict = \"conforme\"\n",
    );

    assert_eq!(
        expectations_that_deviate_without_a_cause(&entries),
        vec!["a_one: autofirma"]
    );
}

#[test]
fn a_cause_that_is_neither_a_card_nor_an_adr_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\ndrive = { mode = \"v4\", script = \"selectcert\" }\n\n[check.expect.autofirma]\nverdict = \"no-conforme\"\ncause = \"BUG-99\"\n\n[check.expect.rfirma]\nverdict = \"no-conforme\"\ncause = \"ADR-0001\"\n",
    );
    let cards: BTreeSet<String> = ["BUG-01"].map(str::to_owned).into();
    let adrs: BTreeSet<String> = ["ADR-0001"].map(str::to_owned).into();

    assert_eq!(
        causes_that_do_not_resolve(&entries, &cards, &adrs),
        vec!["a_one: BUG-99"]
    );
}

#[test]
fn the_reader_of_adrs_takes_the_number_of_each_file() {
    let adrs = adrs_in(&repository_root().join("docs/adr"));
    assert!(adrs.contains("ADR-0001"));
    assert!(!adrs.contains(&format!("ADR-{}", 9999)));
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
fn a_check_that_needs_a_person_without_a_question_is_caught_and_named() {
    let entries = entries_in(
        "[[check]]\nid = \"a_one\"\nneeds = [\"persona\"]\ndrive = { mode = \"v4\", script = \"save\" }\n",
    );
    assert_eq!(person_entries_without_a_question(&entries), vec!["a_one"]);
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
