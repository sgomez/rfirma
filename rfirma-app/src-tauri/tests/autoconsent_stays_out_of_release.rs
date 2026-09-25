//! La feature `conformance-autoconsent` no entra en nada que se publique, y la puerta del contenido busca su interruptor en el paquete.

use std::fs;
use std::path::{Path, PathBuf};

const FEATURE: &str = "conformance-autoconsent";

const SWITCH: &str = "RFIRMA_CONFORMANCE_AUTOCONSENT";

const RELEASE_BUILDERS: [&str; 5] = [
    "packaging/flatpak/me.sgomez.rfirma.yml",
    "rfirma-app/src-tauri/tauri.conf.json",
    ".github/workflows/build.yml",
    ".github/workflows/publish.yml",
    ".github/workflows/release.yml",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative))
        .unwrap_or_else(|error| panic!("{relative} deberia leerse: {error}"))
}

/// Los nombres de las recetas del `justfile` que nombran la feature y no son del grupo `dev`.
fn recipes_outside_dev_naming_the_feature(justfile: &str) -> Vec<String> {
    let mut offenders: Vec<String> = Vec::new();
    let mut in_dev = false;
    let mut current: Option<(String, bool)> = None;
    for line in justfile.lines() {
        if let Some(group) = line
            .strip_prefix("[group('")
            .and_then(|rest| rest.strip_suffix("')]"))
        {
            in_dev = group == "dev";
            continue;
        }
        if is_a_recipe_header(line) {
            let name = line.split([' ', ':']).next().unwrap_or_default().to_owned();
            current = Some((name, in_dev));
            in_dev = false;
        }
        let names_it = line.contains(FEATURE) && !line.trim_start().starts_with('#');
        if let Some((name, false)) = &current {
            if names_it && !offenders.contains(name) {
                offenders.push(name.clone());
            }
        }
    }
    offenders
}

fn is_a_recipe_header(line: &str) -> bool {
    !line.starts_with([' ', '\t', '#', '[']) && !line.contains(":=") && line.contains(':')
}

/// Las líneas que compilan para publicar con todas las features encendidas.
fn release_builds_with_every_feature(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.contains("--release") || line.contains("tauri build"))
        .filter(|line| line.contains("--all-features") || line.contains(FEATURE))
        .map(str::to_owned)
        .collect()
}

#[test]
fn no_release_builder_names_the_feature() {
    for builder in RELEASE_BUILDERS {
        assert!(
            !read(builder).contains(FEATURE),
            "{builder} nombra {FEATURE}: esa feature no entra en nada que se publique"
        );
    }
}

#[test]
fn no_release_build_turns_on_every_feature() {
    for builder in RELEASE_BUILDERS.iter().chain(&["justfile"]) {
        let offenders = release_builds_with_every_feature(&read(builder));
        assert!(
            offenders.is_empty(),
            "{builder} compila para publicar con {FEATURE} dentro:\n  {}",
            offenders.join("\n  ")
        );
    }
}

#[test]
fn only_dev_recipes_name_the_feature() {
    let offenders = recipes_outside_dev_naming_the_feature(&read("justfile"));
    assert!(
        offenders.is_empty(),
        "recetas fuera del grupo dev que nombran {FEATURE}: {offenders:?}"
    );
}

#[test]
fn no_other_feature_turns_it_on() {
    let manifest = read("rfirma-app/src-tauri/Cargo.toml");
    let naming: Vec<&str> = manifest
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && line.contains(FEATURE))
        .collect();
    assert_eq!(naming, [format!("{FEATURE} = []")]);
}

#[test]
fn the_content_gate_and_the_suite_name_the_switch_the_binary_reads() {
    assert!(read("packaging/verifica-contenido.sh").contains(SWITCH));
    assert!(read("rfirma-conformance/src/errand.rs").contains(&format!("= \"{SWITCH}\";")));
    assert!(read("rfirma-app/src-tauri/src/site/adapters/unattended.rs")
        .contains(&format!("const SWITCH: &str = \"{SWITCH}\";")));
}

#[test]
fn the_recipe_reader_catches_a_release_recipe_naming_the_feature() {
    let justfile = format!(
        "[group('ci')]\nbuild-rust: build-ts\n    cargo build --release --features {FEATURE}\n\n\
         [group('dev')]\nfine: build-ts\n    cargo build --profile conformance --features {FEATURE}\n"
    );

    assert_eq!(
        recipes_outside_dev_naming_the_feature(&justfile),
        ["build-rust"]
    );
}

#[test]
fn the_release_reader_catches_every_feature_on_a_release_build() {
    let text = "cargo build --release --all-features\ncargo test --all-features\n\
                pnpm exec tauri build --features conformance-autoconsent\n";

    assert_eq!(release_builds_with_every_feature(text).len(), 2);
}
