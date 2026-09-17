//! Guarda de grada A: cada ficha del anexo A1 tiene caso del sondeo o marca de no
//! observable con motivo, y ningún caso cita una ficha inexistente.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Una ficha del anexo A1 extraída del texto del documento.
#[derive(Debug, Clone, PartialEq, Eq)]
struct A1Card {
    id: String,
    title: String,
    probe_case: Option<String>,
    unobservable: Option<String>,
}

/// Una cita a una ficha de A1 encontrada en el código del sondeo.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Citation {
    file: String,
    line: usize,
    card_id: String,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Parsea las fichas `### BUG-NN:` del anexo A1.
fn parse_a1_cards(annex_text: &str) -> Vec<A1Card> {
    let mut cards = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_title: Option<String> = None;
    let mut current_probe_case: Option<String> = None;
    let mut current_unobservable: Option<String> = None;

    for line in annex_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("### BUG-") {
            if let Some(id) = current_id.take() {
                cards.push(A1Card {
                    id,
                    title: current_title.take().unwrap_or_default(),
                    probe_case: current_probe_case.take(),
                    unobservable: current_unobservable.take(),
                });
            }
            if let Some(colon) = trimmed.find(':') {
                let id = trimmed[4..colon].trim().to_owned();
                let title = trimmed[colon + 1..].trim().to_owned();
                current_id = Some(id);
                current_title = Some(title);
            }
            continue;
        }

        if current_id.is_some() {
            if let Some(at) = trimmed.find("**No observable:**") {
                let motive = trimmed[at + 18..].trim().to_owned();
                current_unobservable = Some(motive);
            } else if let Some(at) = trimmed.find("**Caso del sondeo:**") {
                let case_name = trimmed[at + 20..]
                    .trim()
                    .trim_matches('`')
                    .trim_matches('.')
                    .to_owned();
                current_probe_case = Some(case_name);
            } else if trimmed.contains("**Veredicto del sondeo") {
                if let Some(case_start) = trimmed.find("con `") {
                    let after = &trimmed[case_start + 5..];
                    if let Some(case_end) = after.find('`') {
                        current_probe_case = Some(after[..case_end].to_owned());
                    }
                } else {
                    current_probe_case = Some(String::new());
                }
            }
        }
    }

    if let Some(id) = current_id {
        cards.push(A1Card {
            id,
            title: current_title.unwrap_or_default(),
            probe_case: current_probe_case,
            unobservable: current_unobservable,
        });
    }

    cards
}

/// Extrae las citas a `BUG-NN` en un texto fuente de Rust.
fn bug_citations_in(file_name: &str, text: &str) -> Vec<Citation> {
    let mut citations = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(at) = rest.find("BUG-") {
            let after = &rest[at + 4..];
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            let starts_a_word = at == 0 || !rest[..at].ends_with(|c: char| c.is_alphanumeric());
            if starts_a_word && digits.len() >= 2 {
                citations.push(Citation {
                    file: file_name.to_owned(),
                    line: line_index + 1,
                    card_id: format!("BUG-{digits}"),
                });
            }
            rest = after;
        }
    }
    citations
}

/// Localiza fichas de A1 que no tienen caso asignado ni marca de no observable.
fn find_undecided_cards(cards: &[A1Card], cited_in_probe: &BTreeSet<String>) -> Vec<String> {
    cards
        .iter()
        .filter(|card| {
            let has_case = card.probe_case.is_some() || cited_in_probe.contains(&card.id);
            !has_case && card.unobservable.is_none()
        })
        .map(|card| card.id.clone())
        .collect()
}

/// Localiza fichas marcadas como no observables pero cuyo motivo está vacío.
fn find_unmotivated_cards(cards: &[A1Card]) -> Vec<String> {
    cards
        .iter()
        .filter(|card| {
            if let Some(ref motive) = card.unobservable {
                motive.trim().is_empty()
            } else {
                false
            }
        })
        .map(|card| card.id.clone())
        .collect()
}

/// Localiza citas a fichas de A1 que no existen en el anexo.
fn find_dangling_citations(
    citations: &[Citation],
    existing_cards: &BTreeSet<String>,
) -> Vec<String> {
    citations
        .iter()
        .filter(|citation| !existing_cards.contains(&citation.card_id))
        .map(|citation| format!("{}:{}: {}", citation.file, citation.line, citation.card_id))
        .collect()
}

#[test]
fn every_a1_card_in_annex_is_decided() {
    let annex_path = repository_root().join("docs/afirma/1.9.2/A1-bugs-autofirma.md");
    let probe_cases_path = repository_root().join("rfirma-app/src-tauri/examples/probe/cases.rs");

    let annex_text = std::fs::read_to_string(&annex_path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", annex_path.display()));
    let probe_text = std::fs::read_to_string(&probe_cases_path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", probe_cases_path.display()));

    let cards = parse_a1_cards(&annex_text);
    assert!(
        cards.len() >= 26,
        "el anexo A1 deberia tener al menos 26 fichas; encontradas {}",
        cards.len()
    );

    let citations = bug_citations_in("cases.rs", &probe_text);
    let cited_ids: BTreeSet<String> = citations.into_iter().map(|c| c.card_id).collect();

    let undecided = find_undecided_cards(&cards, &cited_ids);
    assert!(
        undecided.is_empty(),
        "hay fichas de A1 sin caso y sin marca de no observable:\n  {}",
        undecided.join("\n  ")
    );

    let unmotivated = find_unmotivated_cards(&cards);
    assert!(
        unmotivated.is_empty(),
        "hay fichas de A1 con marca de no observable sin motivo:\n  {}",
        unmotivated.join("\n  ")
    );
}

#[test]
fn no_probe_case_cites_a_nonexistent_a1_card() {
    let annex_path = repository_root().join("docs/afirma/1.9.2/A1-bugs-autofirma.md");
    let probe_cases_path = repository_root().join("rfirma-app/src-tauri/examples/probe/cases.rs");

    let annex_text = std::fs::read_to_string(&annex_path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", annex_path.display()));
    let probe_text = std::fs::read_to_string(&probe_cases_path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", probe_cases_path.display()));

    let cards = parse_a1_cards(&annex_text);
    let existing_ids: BTreeSet<String> = cards.into_iter().map(|c| c.id).collect();

    let citations = bug_citations_in("cases.rs", &probe_text);
    let dangling = find_dangling_citations(&citations, &existing_ids);

    assert!(
        dangling.is_empty(),
        "hay casos del sondeo que citan fichas inexistentes de A1:\n  {}",
        dangling.join("\n  ")
    );
}

#[test]
fn a_card_without_case_or_unobservable_mark_fails_and_names_the_card() {
    let annex_snippet = r#"
### BUG-01: Proceso huerfano indefinido
* **Estado en `master`:** Corregido.
"#;
    let cards = parse_a1_cards(annex_snippet);
    let cited = BTreeSet::new();
    let undecided = find_undecided_cards(&cards, &cited);
    assert_eq!(undecided, vec!["BUG-01"]);
}

#[test]
fn a_case_citing_a_nonexistent_card_fails_and_names_the_card() {
    let citations = vec![Citation {
        file: "cases.rs".to_owned(),
        line: 42,
        card_id: "BUG-99".to_owned(),
    }];
    let existing: BTreeSet<String> = ["BUG-01", "BUG-02"].map(str::to_owned).into();
    let dangling = find_dangling_citations(&citations, &existing);
    assert_eq!(dangling, vec!["cases.rs:42: BUG-99"]);
}

#[test]
fn an_unobservable_mark_without_a_reason_fails_and_names_the_card() {
    let annex_snippet = r#"
### BUG-03: Script de terminacion en macOS
* **No observable:**
* **Estado en `master`:** Sigue presente.
"#;
    let cards = parse_a1_cards(annex_snippet);
    let unmotivated = find_unmotivated_cards(&cards);
    assert_eq!(unmotivated, vec!["BUG-03"]);
}

#[test]
fn a_clean_annex_and_cases_pass_with_no_dangling_or_undecided() {
    let annex_snippet = r#"
### BUG-01: Primer defecto
* **Caso del sondeo:** `primer_caso`.
* **Estado en `master`:** Corregido.

### BUG-02: Segundo defecto
* **No observable:** no llega al cable, ocurre en la memoria interna de la JVM.
* **Estado en `master`:** Sigue presente.
"#;
    let cards = parse_a1_cards(annex_snippet);
    let existing: BTreeSet<String> = cards.iter().map(|c| c.id.clone()).collect();
    let cited: BTreeSet<String> = ["BUG-01"].map(str::to_owned).into();
    let citations = vec![Citation {
        file: "cases.rs".to_owned(),
        line: 10,
        card_id: "BUG-01".to_owned(),
    }];

    assert!(find_undecided_cards(&cards, &cited).is_empty());
    assert!(find_unmotivated_cards(&cards).is_empty());
    assert!(find_dangling_citations(&citations, &existing).is_empty());
}

#[test]
fn cases_without_a1_card_are_accepted_if_they_declare_what_they_observe() {
    // Casos de divergencia sin ficha de A1 (como saludo o los de #731) no citan BUG-
    let probe_source = r#"
/// El guion de una sola selección, el más corto que hace saludar al cliente publicado.
const THE_SINGLE_SELECTION: &str = "selectcert";

/// Mide la divergencia en selectcert exigiendo clave privada.
const THE_PRIVATE_KEY_CHECK_CASE: &str = "selectcert_checks_private_key";
"#;
    let citations = bug_citations_in("cases.rs", probe_source);
    // No generan citas dangling porque no mencionan BUG-
    let existing: BTreeSet<String> = ["BUG-01"].map(str::to_owned).into();
    let dangling = find_dangling_citations(&citations, &existing);
    assert!(dangling.is_empty());
}

#[test]
fn probe_cases_include_private_key_check_divergence_case() {
    let probe_cases_path = repository_root().join("rfirma-app/src-tauri/examples/probe/cases.rs");
    let probe_text = std::fs::read_to_string(&probe_cases_path)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", probe_cases_path.display()));
    assert!(
        probe_text.contains("selectcert_checks_private_key"),
        "cases.rs debe incluir el caso de divergencia selectcert_checks_private_key"
    );
}
