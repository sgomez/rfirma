//! Ningún `.rs` de producción del backend declara un `mod tests` interno: las
//! pruebas de un módulo viven en su fichero hermano `tests.rs` (issue #444).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Ficheros `.rs` de producción versionados bajo `src/`, excluidos los hermanos de pruebas.
fn production_files(root: &Path) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", "rfirma-app/src-tauri/src"])
        .current_dir(root)
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");

    String::from_utf8(listing.stdout)
        .expect("las rutas deberian ser UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .filter(|path| path.ends_with(".rs"))
        .filter(|path| !path.ends_with("/tests.rs") && !path.ends_with("\\tests.rs"))
        .map(str::to_owned)
        .collect()
}

/// Comprueba si la línea abre un módulo de pruebas interno.
fn declares_an_inline_test_module(line: &str) -> bool {
    line.trim_end() == "mod tests {"
}

#[test]
fn no_production_file_declares_an_inline_test_module() {
    let root = repository_root();
    let files = production_files(&root);
    assert!(
        files.len() > 50,
        "el listado no ha encontrado el codigo del backend: {} ficheros",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for relative in &files {
        let contents = fs::read_to_string(root.join(relative))
            .unwrap_or_else(|error| panic!("deberia leerse {relative}: {error}"));
        for (number, line) in contents.lines().enumerate() {
            if declares_an_inline_test_module(line) {
                offenders.push(format!("{relative}:{}", number + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "el issue #444 saca las pruebas de cada modulo a su fichero hermano `tests.rs`; \
         estos ficheros vuelven a declarar `mod tests` por dentro:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_needle_catches_the_declaration_and_leaves_the_reference_alone() {
    assert!(declares_an_inline_test_module("mod tests {"));
    assert!(!declares_an_inline_test_module("mod tests;"));
    assert!(!declares_an_inline_test_module("// mod tests {"));
    assert!(!declares_an_inline_test_module("pub mod tests_helpers {"));
}
