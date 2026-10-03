//! Guarda de los índices: en el backend y en la interfaz cada módulo abre con una cabecera `//!` que dice qué es, y `just outline <directorio>/` las junta (ADR-0017).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Zona de código cuyos módulos abren con cabecera.
struct Zone {
    /// Raíz de la zona, relativa a la raíz del repositorio.
    root: &'static str,
    /// Extensiones que cuentan como módulo.
    extensions: &'static [&'static str],
}

const ZONES: [Zone; 2] = [
    Zone {
        root: "rfirma-app/src-tauri/src",
        extensions: &["rs"],
    },
    Zone {
        root: "rfirma-app/src",
        extensions: &["ts", "tsx"],
    },
];

/// Comprueba si el fichero es de pruebas, al que no se le pide cabecera; `scripts/outline.sh` usa el mismo criterio.
fn is_a_test_file(relative: &str) -> bool {
    let rooted = format!("/{relative}");
    let a_rust_test = relative.ends_with(".rs")
        && (rooted.ends_with("/tests.rs")
            || rooted.ends_with("_tests.rs")
            || rooted.contains("/tests/"));
    a_rust_test || relative.ends_with(".test.ts") || relative.ends_with(".test.tsx")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Módulos versionados de una zona devueltos por git.
fn tracked_modules(root: &Path, zone: &Zone) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", zone.root])
        .current_dir(root)
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");

    let modules: Vec<String> = String::from_utf8(listing.stdout)
        .expect("las rutas deberian ser UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| {
            zone.extensions
                .iter()
                .any(|extension| path.ends_with(&format!(".{extension}")))
        })
        .map(|path| {
            path.strip_prefix(&format!("{}/", zone.root))
                .expect("git deberia devolver rutas dentro de la zona pedida")
                .to_owned()
        })
        .filter(|relative| !is_a_test_file(relative))
        .collect();
    assert!(
        modules.len() > 5,
        "el listado no ha encontrado el codigo de {}: {} ficheros",
        zone.root,
        modules.len()
    );
    modules
}

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path))
        .unwrap_or_else(|error| panic!("deberia leerse {path}: {error}"))
}

/// Lo que sobra o falta en la cabecera con la que abre un módulo, si algo.
fn what_is_wrong_with_the_header(source: &str) -> Option<String> {
    let mut lines = source.lines();
    let Some(header) = lines
        .next()
        .and_then(|first| first.strip_prefix("//!"))
        .map(str::trim)
        .filter(|header| !header.is_empty())
    else {
        return Some("no abre con una linea `//!` que diga que es".to_owned());
    };
    let spills_over = lines
        .next()
        .and_then(|second| second.strip_prefix("//!"))
        .is_some_and(|rest| !rest.trim().is_empty());
    if spills_over {
        return Some(
            "la cabecera sigue en la segunda linea: una frase, en una sola linea `//!`".to_owned(),
        );
    }
    what_is_wrong_with(header)
}

#[test]
fn every_module_opens_with_a_header_that_says_what_it_is() {
    let root = repository_root();

    for zone in &ZONES {
        let wrong: Vec<String> = tracked_modules(&root, zone)
            .iter()
            .filter_map(|relative| {
                let source = read(&root, &format!("{}/{relative}", zone.root));
                what_is_wrong_with_the_header(&source)
                    .map(|reason| format!("{relative}\n  {reason}"))
            })
            .collect();
        assert!(
            wrong.is_empty(),
            "en {} el indice de modulos lo da `just outline <directorio>/` con la primera linea \
             `//!` de cada uno: una frase que dice que es el modulo y, si ayuda, que no es; el \
             como lo dice el codigo y el porque un ADR:\n\n{}",
            zone.root,
            wrong.join("\n\n")
        );
    }
}

#[test]
fn the_tests_of_the_window_are_not_asked_for_a_header() {
    assert!(is_a_test_file("signing/flow.test.ts"));
    assert!(is_a_test_file("App.test.tsx"));
    assert!(!is_a_test_file("signing/flow.ts"));
    assert!(!is_a_test_file("testing/render.tsx"));
    assert!(
        !is_a_test_file("tests/render.tsx"),
        "una carpeta `tests/` solo aparta ficheros de Rust"
    );
}

#[test]
fn the_tests_of_the_backend_are_not_asked_for_a_header() {
    assert!(is_a_test_file("site/adapters/codec/tests.rs"));
    assert!(is_a_test_file("site/application/errand/tests/support.rs"));
    assert!(is_a_test_file(
        "desktop/adapters/firefox_lock/windows_tests.rs"
    ));
    assert!(is_a_test_file("tests.rs"));
    assert!(!is_a_test_file("site/adapters/header_probe.rs"));
    assert!(!is_a_test_file("crossing/guards.rs"));
    assert!(!is_a_test_file("site/domain/protocol/testsuite.rs"));
}

#[test]
fn a_module_without_a_header_is_caught() {
    assert!(what_is_wrong_with_the_header("use std::fs;\n").is_some());
    assert!(
        what_is_wrong_with_the_header("// Sin consola.\n//! Llega tarde.\n").is_some(),
        "la cabecera es la primera linea, no una `//!` cualquiera"
    );
    assert!(
        what_is_wrong_with_the_header("/** El flujo de firma. */\nexport {};\n").is_some(),
        "un JSDoc no es la cabecera"
    );
    assert!(what_is_wrong_with_the_header("//!\n//! Despues del hueco.\n").is_some());
    assert!(what_is_wrong_with_the_header("").is_some());
}

#[test]
fn a_header_is_one_line_and_the_prose_below_it_is_free() {
    assert!(what_is_wrong_with_the_header("//! Una frase\n//! partida en dos.\n").is_some());
    assert_eq!(
        what_is_wrong_with_the_header(
            "//! Las reglas puras de la CA local (ADR-0005).\n//!\n//! Prosa aparte.\n"
        ),
        None
    );
    assert_eq!(
        what_is_wrong_with_the_header("//! El canal local.\n\nuse std::fs;\n"),
        None
    );
    assert_eq!(
        what_is_wrong_with_the_header(
            "//! Las tres etapas de la firma.\n/**\n * Prosa aparte.\n */\nimport x from \"y\";\n"
        ),
        None
    );
}

#[test]
fn a_header_that_cites_the_spec_or_explains_how_it_works_is_caught() {
    assert!(what_is_wrong_with_the_header("//! Lo que ya no compila (#439, #441).\n").is_some());
    assert!(what_is_wrong_with_the_header("//! Lo que sea (ID-215).\n").is_some());
    let long = format!("//! {}\n", "y ademas ".repeat(40));
    assert!(what_is_wrong_with_the_header(&long).is_some());
    assert_eq!(
        what_is_wrong_with_the_header("//! El modulo PKCS#11 del sistema (ADR-0022).\n"),
        None
    );
}

/// Lo que cabe en una cabecera: una frase que dice qué es el fichero.
const THE_LONGEST_HEADER: usize = 300;

/// Prefijos de los identificadores de la especificación, que mueren con ella.
const SPEC_CITATIONS: [&str; 4] = ["ID-", "TD-", "RD-", "RT-"];

/// Comprueba si el texto cita un identificador de la especificación o un número de issue.
fn cites_something_that_dies(header: &str) -> bool {
    let dies_after = |prefix: &str| {
        header
            .match_indices(prefix)
            .any(|(at, _)| header[at + prefix.len()..].starts_with(|c: char| c.is_ascii_digit()))
    };
    let is_an_issue = header.match_indices('#').any(|(at, _)| {
        let before_it_starts = at == 0 || matches!(&header[at - 1..at], " " | "(");
        before_it_starts && header[at + 1..].starts_with(|c: char| c.is_ascii_digit())
    });

    SPEC_CITATIONS.iter().any(|prefix| dies_after(prefix)) || is_an_issue
}

/// Lo que sobra de una cabecera, si sobra algo.
fn what_is_wrong_with(header: &str) -> Option<String> {
    if cites_something_that_dies(header) {
        return Some(
            "cita un ID-NN o un numero de issue, que mueren antes que el codigo".to_owned(),
        );
    }
    let length = header.chars().count();
    if length > THE_LONGEST_HEADER {
        return Some(format!(
            "son {length} caracteres: a partir de {THE_LONGEST_HEADER} ya no dice que es, cuenta como funciona"
        ));
    }
    None
}

#[test]
fn what_counts_as_a_citation_that_dies() {
    assert!(cites_something_that_dies("Lo que sea (ID-215)."));
    assert!(cites_something_that_dies("Lo que sea (TD-9)."));
    assert!(cites_something_that_dies("Lo que sea, del #453."));
    assert!(!cites_something_that_dies("Lo que sea (ADR-0017)."));
    assert!(!cites_something_that_dies(
        "El identificador RD del formulario."
    ));
    assert!(
        !cites_something_that_dies("El modulo PKCS#11 del sistema."),
        "PKCS#11 no es un numero de issue"
    );
}
