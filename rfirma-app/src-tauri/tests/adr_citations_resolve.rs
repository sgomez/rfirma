//! Cada `ADR-NNNN` citado en un `.rs` del backend tiene su fichero en `docs/adr/`, y esta guarda lo comprueba leyendo el código como texto.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Una cita a un ADR y dónde está.
#[derive(Debug, PartialEq, Eq)]
struct Citation {
    file: String,
    line: usize,
    number: String,
}

/// Un fichero fuente y su contenido, para que la guarda sirva sobre un árbol sintético.
struct Source<'a> {
    path: &'a str,
    text: &'a str,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Los `.rs` **versionados** del backend, preguntándoselo a git.
fn tracked_rust_files() -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", "--", "*.rs"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");
    String::from_utf8(listing.stdout)
        .expect("las rutas deberian ser UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Los números de ADR con fichero en `docs/adr/`.
fn adr_numbers_in(adr_dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(adr_dir)
        .expect("docs/adr deberia existir")
        .map(|entry| entry.expect("cada entrada deberia leerse").file_name())
        .filter_map(|name| name.to_str().map(str::to_owned))
        .filter(|name| name.ends_with(".md"))
        .filter_map(|name| name.split('-').next().map(str::to_owned))
        .filter(|prefix| prefix.len() == 4 && prefix.chars().all(|c| c.is_ascii_digit()))
        .collect()
}

fn adr_numbers_cited_in(line: &str) -> Vec<String> {
    let mut cited = Vec::new();
    let mut rest = line;
    while let Some(at) = rest.find("ADR-") {
        let after = &rest[at + 4..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        let starts_a_word = at == 0 || !rest[..at].ends_with(|c: char| c.is_alphanumeric());
        if starts_a_word && digits.len() == 4 {
            cited.push(digits);
        }
        rest = after;
    }
    cited
}

fn citations_in(sources: &[Source]) -> Vec<Citation> {
    sources
        .iter()
        .flat_map(|source| {
            source
                .text
                .lines()
                .enumerate()
                .flat_map(move |(index, line)| {
                    adr_numbers_cited_in(line)
                        .into_iter()
                        .map(move |number| Citation {
                            file: source.path.to_owned(),
                            line: index + 1,
                            number,
                        })
                })
        })
        .collect()
}

/// Las citas cuyo ADR no existe, escritas como `fichero:línea: ADR-NNNN`.
fn dangling_citations(sources: &[Source], existing: &BTreeSet<String>) -> Vec<String> {
    citations_in(sources)
        .into_iter()
        .filter(|citation| !existing.contains(&citation.number))
        .map(|citation| {
            format!(
                "{}:{}: ADR-{}",
                citation.file, citation.line, citation.number
            )
        })
        .collect()
}

/// Ruta y texto de cada `.rs` versionado del backend.
fn backend_sources() -> Vec<(String, String)> {
    let files = tracked_rust_files();
    assert!(
        files.len() > 50,
        "el backend tiene mas de cincuenta .rs; git ha listado {}",
        files.len()
    );
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(&path))
                .unwrap_or_else(|error| panic!("no se pudo leer {path}: {error}"));
            (path, text)
        })
        .collect()
}

fn as_sources(owned: &[(String, String)]) -> Vec<Source<'_>> {
    owned
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect()
}

#[test]
fn every_adr_cited_in_the_backend_has_a_file() {
    let existing = adr_numbers_in(&repository_root().join("docs/adr"));
    assert!(
        existing.len() >= 18,
        "docs/adr deberia tener al menos dieciocho ADR"
    );
    let owned = backend_sources();

    let dangling = dangling_citations(&as_sources(&owned), &existing);
    assert!(
        dangling.is_empty(),
        "hay citas a ADR sin fichero en docs/adr/:\n  {}",
        dangling.join("\n  ")
    );
}

#[test]
fn the_backend_cites_at_least_one_adr_so_the_guard_has_work() {
    let owned = backend_sources();

    assert!(citations_in(&as_sources(&owned)).len() > 10);
}

#[test]
fn a_citation_without_a_file_is_caught_and_named_with_file_and_line() {
    let existing: BTreeSet<String> = ["0001", "0005"].map(str::to_owned).into();
    let sources = [
        Source {
            path: "src/fine.rs",
            text: "// Como dice el ADR-0001.\nfn a() {}\n",
        },
        Source {
            path: "src/broken.rs",
            text: concat!(
                "fn b() {}\n\n/// Ver ",
                "ADR-",
                "0099 y ADR-0005.\nfn c() {}\n"
            ),
        },
    ];

    assert_eq!(
        dangling_citations(&sources, &existing),
        vec![concat!("src/broken.rs:3: ", "ADR-", "0099")]
    );
}

#[test]
fn a_clean_tree_reports_nothing() {
    let existing: BTreeSet<String> = ["0001"].map(str::to_owned).into();
    let sources = [Source {
        path: "src/fine.rs",
        text: "// ADR-0001\n",
    }];

    assert!(dangling_citations(&sources, &existing).is_empty());
}

#[test]
fn the_reader_only_takes_four_digit_numbers_that_start_a_word() {
    assert_eq!(
        adr_numbers_cited_in("ADR-0005, (ADR-0012) y ADR-0014."),
        ["0005", "0012", "0014"]
    );
    assert!(adr_numbers_cited_in("XADR-0005 ADR-12 ADR- ADR-abcd").is_empty());
    assert_eq!(adr_numbers_cited_in("ADR-00123"), Vec::<String>::new());
}

#[test]
fn the_adr_directory_is_read_by_its_four_digit_prefix() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "0001-a.md",
        "0042-b.md",
        "README.md",
        "12-c.md",
        "0007-d.txt",
    ] {
        std::fs::write(dir.path().join(name), "").unwrap();
    }

    assert_eq!(
        adr_numbers_in(dir.path()),
        ["0001", "0042"].map(str::to_owned).into()
    );
}
