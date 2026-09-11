//! Las salidas de cobertura del `justfile` caen bajo el directorio de compilación, una por receta, y `clean-coverage` solo borra lo instrumentado; se comprueba ejecutando `just` sobre un árbol sintético.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

fn just(arguments: &[&str]) -> Output {
    Command::new("just")
        .arg("--justfile")
        .arg(repository_root().join("justfile"))
        .args(arguments)
        .current_dir(repository_root())
        .output()
        .expect("just deberia estar: `just tools` lo exige")
}

fn recipes_writing_coverage() -> Vec<String> {
    let justfile =
        std::fs::read_to_string(repository_root().join("justfile")).expect("justfile legible");
    let mut recipes = Vec::new();
    let mut current = None;
    for line in justfile.lines() {
        let starts_a_recipe = line
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase())
            && line.contains(':')
            && !line.contains(":=");
        if starts_a_recipe {
            current = line.split([' ', ':']).next().map(str::to_owned);
        }
        if line.contains("--output-path") {
            if let Some(recipe) = current.clone() {
                if !recipes.contains(&recipe) {
                    recipes.push(recipe);
                }
            }
        }
    }
    recipes
}

fn report_destination_in_a_dry_run(recipe: &str, build_directory: &Path) -> String {
    let setting = build_directory.to_str().expect("ruta UTF-8");
    let output = just(&["--dry-run", "--set", "cargo_target", setting, recipe]);
    let printed = String::from_utf8_lossy(&output.stderr);
    let destination = printed
        .lines()
        .filter_map(|line| line.split("--output-path").nth(1))
        .next_back()
        .unwrap_or_else(|| panic!("`{recipe}` deberia pasar --output-path:\n{printed}"));
    destination
        .split_whitespace()
        .next()
        .expect("--output-path lleva ruta")
        .trim_matches(['"', '\''])
        .to_owned()
}

#[test]
fn every_coverage_report_lands_under_the_build_directory() {
    let build_directory = tempfile::tempdir().expect("directorio temporal");
    let recipes = recipes_writing_coverage();
    assert!(recipes.len() >= 2, "{recipes:?}");

    for recipe in recipes {
        let destination = report_destination_in_a_dry_run(&recipe, build_directory.path());
        assert!(
            Path::new(&destination).starts_with(build_directory.path()),
            "`{recipe}` escribe su informe en {destination}, fuera del directorio de compilación"
        );
    }
}

#[test]
fn no_two_recipes_share_a_coverage_report() {
    let build_directory = tempfile::tempdir().expect("directorio temporal");
    let mut owners: BTreeMap<String, String> = BTreeMap::new();

    for recipe in recipes_writing_coverage() {
        let destination = report_destination_in_a_dry_run(&recipe, build_directory.path());
        if let Some(previous) = owners.insert(destination.clone(), recipe.clone()) {
            panic!("`{previous}` y `{recipe}` escriben los dos en {destination}");
        }
    }
}

#[test]
fn a_justfile_called_from_under_coverage_builds_in_the_instrumented_tree() {
    let build_directory = tempfile::tempdir().expect("directorio temporal");
    let output = Command::new("just")
        .arg("--justfile")
        .arg(repository_root().join("justfile"))
        .args(["--set", "cargo_target"])
        .arg(build_directory.path())
        .args(["--evaluate", "CARGO_TARGET_DIR"])
        .current_dir(repository_root())
        .env("CARGO_LLVM_COV", "1")
        .output()
        .expect("just deberia estar");

    assert_eq!(
        PathBuf::from(String::from_utf8_lossy(&output.stdout).into_owned()),
        build_directory.path().join("llvm-cov-target")
    );
}

fn touch(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("tiene carpeta")).expect("carpeta");
    std::fs::write(path, b"").expect("fichero");
}

#[test]
fn clean_coverage_frees_the_instrumented_tree_and_leaves_the_normal_build_standing() {
    let build_directory = tempfile::tempdir().expect("directorio temporal");
    let sources = tempfile::tempdir().expect("directorio temporal");
    let build = build_directory.path();
    let normal_binary = build.join("debug/rfirma");
    let normal_dependency = build.join("debug/deps/libtauri.rlib");
    let instrumented_binary = build.join("llvm-cov-target/debug/rfirma");
    let stray_dump = sources.path().join("default_1_0_2.profraw");
    let source_file = sources.path().join("Cargo.toml");
    for path in [
        &normal_binary,
        &normal_dependency,
        &instrumented_binary,
        &stray_dump,
        &source_file,
    ] {
        touch(path);
    }

    let output = just(&[
        "--set",
        "cargo_target",
        build.to_str().expect("ruta UTF-8"),
        "--set",
        "tauri",
        sources.path().to_str().expect("ruta UTF-8"),
        "clean-coverage",
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(normal_binary.exists(), "ha borrado la compilación normal");
    assert!(normal_dependency.exists(), "ha borrado las dependencias");
    assert!(source_file.exists(), "ha borrado un fuente");
    assert!(
        !instrumented_binary.exists(),
        "el árbol instrumentado sigue"
    );
    assert!(!stray_dump.exists(), "el volcado suelto sigue");
}
