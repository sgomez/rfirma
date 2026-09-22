//! El catálogo y la referencia se cruzan con la documentación de `docs/afirma/1.9.2/`: capítulos,
//! tabla SAF del capítulo 15, fichas del anexo A1 y su registro de bugs.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rfirma_conformance::catalogue::read_the_catalogue;
use rfirma_conformance::known_bug::{the_known_bugs, InMaster};

#[derive(Debug, Default)]
struct Entry {
    id: String,
    chapter: String,
    declared: String,
    cited_cards: BTreeSet<String>,
    bug: Option<String>,
}

#[derive(Debug)]
struct Known {
    id: String,
    outcome: String,
    cause: String,
}

#[derive(Debug, PartialEq, Eq)]
struct A1Card {
    id: String,
    title: String,
    unobservable: Option<String>,
    master: Option<String>,
}

const THE_KNOWN_OUTCOMES: [&str; 2] = ["conforme", "no-conforme"];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn the_manual() -> PathBuf {
    crate_dir().join("../docs/afirma/1.9.2")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", path.display()))
}

fn tomls_in(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("toml"))
        .collect();
    files.sort();
    files
}

fn string_of(value: &toml::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn an_entry(id: &str, chapter: &str, declared: &str) -> Entry {
    Entry {
        id: id.to_owned(),
        chapter: chapter.to_owned(),
        declared: declared.to_owned(),
        cited_cards: cards_cited_in(declared),
        bug: None,
    }
}

fn known_in(reference: &str) -> Vec<Known> {
    let parsed: toml::Table = toml::from_str(reference)
        .unwrap_or_else(|error| panic!("la referencia no es TOML válido: {error}"));
    parsed
        .get("known")
        .and_then(toml::Value::as_array)
        .map(|known| {
            known
                .iter()
                .map(|value| Known {
                    id: string_of(value, "id"),
                    outcome: string_of(value, "outcome"),
                    cause: string_of(value, "cause"),
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

/// Lo que la tabla del capítulo 15 dice de un código que el original nunca emite: se documenta en
/// el manual y no se le exige a ningún cliente.
const THE_MARKS_OF_A_CODE_NEVER_EMITTED: [&str; 2] = ["*Huérfano*", "*Sin emisor*"];

/// Los códigos que el original emite de la tabla sinóptica del capítulo 15, por las filas que abren
/// con uno.
fn saf_codes_in_the_table(chapter: &str) -> BTreeSet<String> {
    chapter
        .lines()
        .filter(|line| {
            !THE_MARKS_OF_A_CODE_NEVER_EMITTED
                .iter()
                .any(|mark| line.contains(mark))
        })
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
                    title: rest[colon + 1..].replace(['`', '*'], "").trim().to_owned(),
                    unobservable: None,
                    master: None,
                });
            }
            continue;
        }
        let Some(card) = cards.last_mut() else {
            continue;
        };
        if let Some(motive) = what_follows(trimmed, "**No observable:**") {
            card.unobservable = Some(motive.trim().to_owned());
        }
        if let Some(state) = what_follows(trimmed, "**Estado en `master`:** **") {
            card.master = state.split("**").next().map(str::to_owned);
        }
    }
    cards
}

fn what_follows<'a>(line: &'a str, mark: &str) -> Option<&'a str> {
    line.find(mark).map(|at| &line[at + mark.len()..])
}

/// El estado en `master` que el registro da a lo que la ficha dice en prosa, o `None` si la
/// prosa no es de ninguno de los tres.
fn the_state_in_master_of(prose: &str) -> Option<InMaster> {
    if prose.starts_with("Sigue presente") {
        Some(InMaster::Present)
    } else if prose.starts_with("Corregido a medias") || prose.starts_with("Corregido solo") {
        Some(InMaster::Partial)
    } else if prose.starts_with("Corregido") {
        Some(InMaster::Fixed)
    } else {
        None
    }
}

/// Las fichas del anexo tal y como las tiene que tener el registro: id, título y estado en `master`.
fn the_registry_the_annex_asks_for(cards: &[A1Card]) -> Vec<(String, String, Option<InMaster>)> {
    cards
        .iter()
        .map(|card| {
            let master = card.master.as_deref().and_then(the_state_in_master_of);
            (card.id.clone(), card.title.clone(), master)
        })
        .collect()
}

/// Las comprobaciones en que el catálogo y la referencia no dicen lo mismo del bug por el que el
/// original falla: un resultado no conforme se explica con el bug que la comprobación declara, y una
/// comprobación que declara un bug no puede estar prevista como conforme.
fn bugs_out_of_step(known: &[Known], entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| {
            let known = known.iter().find(|known| known.id == entry.id)?;
            let declared = entry.bug.as_deref().unwrap_or("ningún bug");
            if known.outcome == "no-conforme" && entry.bug.as_deref() != Some(&known.cause) {
                Some(format!(
                    "{}: la referencia lo explica con {} y el catálogo declara {declared}",
                    entry.id, known.cause
                ))
            } else if known.outcome != "no-conforme" && entry.bug.is_some() {
                Some(format!(
                    "{}: declara {declared} y la referencia lo da {}",
                    entry.id, known.outcome
                ))
            } else {
                None
            }
        })
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

fn codes_of_the_table_without_an_entry(
    table: &BTreeSet<String>,
    named: &BTreeSet<String>,
) -> Vec<String> {
    table.difference(named).cloned().collect()
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

fn the_catalogue() -> Vec<Entry> {
    read_the_catalogue()
        .unwrap_or_else(|complaint| panic!("{complaint}"))
        .iter()
        .map(|check| Entry {
            bug: check.bug.map(|bug| bug.id.clone()),
            ..an_entry(&check.id, &check.chapter, &check.the_declared_text())
        })
        .collect()
}

fn the_references() -> Vec<String> {
    tomls_in(&crate_dir().join("reference"))
        .iter()
        .map(|path| read(path))
        .collect()
}

fn the_a1_cards() -> Vec<A1Card> {
    a1_cards_in(&read(&the_manual().join("A1-bugs-autofirma.md")))
}

#[test]
fn every_check_cites_a_chapter_with_a_file_in_the_manual() {
    let entries = the_catalogue();
    let chapters = chapters_in(&the_manual());

    assert!(
        entries_whose_chapter_has_no_file(&entries, &chapters).is_empty(),
        "hay entradas que citan un capítulo sin fichero en docs/afirma/1.9.2/:\n  {}",
        entries_whose_chapter_has_no_file(&entries, &chapters).join("\n  ")
    );
}

#[test]
fn every_a1_card_is_decided_and_no_check_cites_one_that_does_not_exist() {
    let entries = the_catalogue();
    let cards = the_a1_cards();
    assert!(
        cards.len() >= 26,
        "el anexo A1 debería tener al menos 26 fichas; encontradas {}",
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
        "hay fichas de A1 sin citar desde el catálogo y sin marca de no observable:\n  {}",
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
    let table = saf_codes_in_the_table(&read(&the_manual().join("15-errores.md")));
    let named: BTreeSet<String> = the_catalogue()
        .iter()
        .flat_map(|entry| saf_codes_named_in(&entry.declared))
        .collect();

    assert_eq!(
        table.len(),
        44,
        "la tabla del capítulo 15 debería tener 44 códigos que el original emite"
    );
    assert!(
        codes_of_the_table_without_an_entry(&table, &named).is_empty(),
        "hay códigos de la tabla del capítulo 15 sin entrada, sin familia y sin declararse no \
         medibles:\n  {}",
        codes_of_the_table_without_an_entry(&table, &named).join("\n  ")
    );
}

#[test]
fn every_known_result_of_the_reference_names_a_check_of_the_catalogue_and_a_card_of_a1() {
    let entries = the_catalogue();
    let cards: BTreeSet<String> = the_a1_cards().into_iter().map(|card| card.id).collect();
    let known: Vec<Known> = the_references()
        .iter()
        .flat_map(|raw| known_in(raw))
        .collect();

    assert!(
        !known.is_empty(),
        "debería haber al menos una referencia con resultados previstos"
    );
    assert!(
        known_results_that_are_wrong(&known, &entries, &cards).is_empty(),
        "hay resultados previstos de la referencia que no cuadran:\n  {}",
        known_results_that_are_wrong(&known, &entries, &cards).join("\n  ")
    );
}

#[test]
fn the_registry_of_known_bugs_is_the_a1_annex_with_its_state_in_master() {
    let registry: Vec<(String, String, Option<InMaster>)> = the_known_bugs()
        .iter()
        .map(|bug| (bug.id.clone(), bug.title.clone(), Some(bug.master)))
        .collect();

    assert_eq!(registry, the_registry_the_annex_asks_for(&the_a1_cards()));
}

#[test]
fn every_failure_the_reference_explains_by_a_bug_is_the_bug_its_check_declares() {
    let entries = the_catalogue();
    let known: Vec<Known> = the_references()
        .iter()
        .flat_map(|raw| known_in(raw))
        .collect();

    assert!(
        bugs_out_of_step(&known, &entries).is_empty(),
        "el catálogo y la referencia no dicen lo mismo del bug:\n  {}",
        bugs_out_of_step(&known, &entries).join("\n  ")
    );
}

#[test]
fn a_card_gets_its_title_without_markdown_and_its_state_in_master_from_its_prose() {
    let cards = a1_cards_in(
        "### BUG-04: Un `load` *roto*\n* **Estado en `master`:** **Corregido a medias.** Algo.\n\n\
         ### BUG-05: Otro\n* **Estado en `master`:** **Sigue presente.**\n\n\
         ### BUG-06: Y otro\n* **Estado en `master`:** **Corregido por cambio de arquitectura.**\n\n\
         ### BUG-07: Sin estado\n",
    );

    assert_eq!(
        the_registry_the_annex_asks_for(&cards),
        [
            ("BUG-04", "Un load roto", Some(InMaster::Partial)),
            ("BUG-05", "Otro", Some(InMaster::Present)),
            ("BUG-06", "Y otro", Some(InMaster::Fixed)),
            ("BUG-07", "Sin estado", None),
        ]
        .map(|(id, title, master)| (id.to_owned(), title.to_owned(), master))
    );
}

#[test]
fn a_bug_the_catalogue_and_the_reference_disagree_on_is_caught_and_named() {
    let known = known_in(
        "[[known]]\nid = \"a_one\"\noutcome = \"no-conforme\"\ncause = \"BUG-01\"\nnote = \"x\"\n\n\
         [[known]]\nid = \"a_two\"\noutcome = \"conforme\"\ncause = \"BUG-02\"\nnote = \"x\"\n\n\
         [[known]]\nid = \"a_three\"\noutcome = \"no-conforme\"\ncause = \"BUG-03\"\nnote = \"x\"\n",
    );
    let declaring = |id: &str, bug: Option<&str>| Entry {
        bug: bug.map(str::to_owned),
        ..an_entry(id, "05", "")
    };
    let entries = [
        declaring("a_one", None),
        declaring("a_two", Some("BUG-02")),
        declaring("a_three", Some("BUG-03")),
        declaring("a_four", Some("BUG-04")),
    ];

    assert_eq!(
        bugs_out_of_step(&known, &entries),
        [
            "a_one: la referencia lo explica con BUG-01 y el catálogo declara ningún bug",
            "a_two: declara BUG-02 y la referencia lo da conforme",
        ]
    );
}

#[test]
fn a_chapter_without_a_file_is_caught_and_named() {
    let entries = [an_entry("a_one", "99", "")];
    let chapters: BTreeSet<String> = ["05"].map(str::to_owned).into();

    assert_eq!(
        entries_whose_chapter_has_no_file(&entries, &chapters),
        vec!["a_one: 99"]
    );
}

#[test]
fn a_code_the_original_emits_and_the_catalogue_never_names_is_caught_and_named() {
    let table = saf_codes_in_the_table(
        "| `SAF_00` | `ERROR_CANNOT_READ_DATA` |\n| `SAF_07` | `ERROR_CANNOT_FIND_KEYSTORE` |\n\
         | `SAF_10` | `ERROR_NO_CERTIFICATES_SYSTEM` | *Huérfano* (no referenciado) |\n",
    );
    let named = saf_codes_named_in("saf = \"SAF_00\"\nstatement = \"SAF_070 no cuenta.\"");

    assert_eq!(
        codes_of_the_table_without_an_entry(&table, &named),
        vec!["SAF_07"]
    );
}

#[test]
fn a_known_result_outside_the_catalogue_the_vocabulary_or_the_annex_is_caught_and_named() {
    let entries = [an_entry("a_one", "05", "")];
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
fn a_cited_card_that_does_not_exist_and_an_undecided_one_are_both_caught_and_named() {
    let entries = [an_entry("a_one", "05", "Lo explica BUG-99.")];
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
