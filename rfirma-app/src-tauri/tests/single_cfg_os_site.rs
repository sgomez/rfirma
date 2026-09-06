//! `paths.rs` es el único fichero del repositorio con un condicional de sistema operativo (ADR-0010).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// El único fichero autorizado, relativo a la raíz del repositorio.
const THE_ONLY_SITE: &str = "rfirma-app/src-tauri/src/paths.rs";

/// Fichero de esta prueba para no acusarse a sí misma.
const THIS_TEST: &str = "rfirma-app/src-tauri/tests/single_cfg_os_site.rs";

/// Patrones para detectar condicionales de compilación por sistema operativo.
const NEEDLES: [&str; 5] = ["cfg", "target_os", "target_family", "(unix", "(windows"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Ficheros `.rs` versionados devueltos por git.
fn tracked_rust_files(root: &Path) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", "*.rs"])
        .current_dir(root)
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

/// Comprueba si la línea contiene un condicional de sistema operativo.
fn conditions_the_operating_system(line: &str) -> bool {
    line.contains(NEEDLES[1])
        || line.contains(NEEDLES[2])
        || (line.contains(NEEDLES[0]) && (line.contains(NEEDLES[3]) || line.contains(NEEDLES[4])))
}

#[test]
fn paths_rs_is_the_only_file_in_the_repository_that_knows_the_operating_system() {
    let root = repository_root();
    let files = tracked_rust_files(&root);
    assert!(
        files.len() > 5,
        "el listado no ha encontrado el codigo: {} ficheros",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for relative in &files {
        if relative == THE_ONLY_SITE || relative == THIS_TEST {
            continue;
        }
        let contents = fs::read_to_string(root.join(relative)).unwrap_or_else(|error| {
            panic!("deberia leerse {relative}: {error}");
        });
        for (number, line) in contents.lines().enumerate() {
            if conditions_the_operating_system(line) {
                offenders.push(format!("{relative}:{}: {}", number + 1, line.trim()));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "el ADR-0010 pone TODO el conocimiento del sistema operativo en {THE_ONLY_SITE}, \
         y estas lineas lo sacan de ahi:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_authorised_file_is_there_and_really_carries_the_conditional() {
    let contents = fs::read_to_string(repository_root().join(THE_ONLY_SITE))
        .expect("paths.rs deberia estar donde dice el ADR-0010");

    assert!(
        contents.lines().any(conditions_the_operating_system),
        "si paths.rs ya no decide por sistema operativo, esta prueba se ha quedado sin objeto"
    );
}

/// Formas de condicional de compilación que la guarda debe cazar.
const THE_FOUR_FORMS: [&str; 6] = [
    r#"#[cfg(target_os = "windows")]"#,
    r#"#[cfg(target_family = "unix")]"#,
    "    if cfg!(unix) { uno() } else { otro() }",
    "#[cfg(not(unix))]",
    r#"#[cfg(any(unix, target_env = "musl"))]"#,
    "#[cfg(windows)]",
];

/// Líneas que no deben disparar la guarda.
const WHAT_MUST_NOT_TRIP_IT: [&str; 4] = [
    "#[cfg(test)]",
    r#"#[cfg(feature = "flatpak")]"#,
    "/// El comportamiento difiere entre (unix) y (windows).",
    "// windows, unix: las dos familias se comportan igual aqui.",
];

#[test]
fn the_needles_catch_every_form_the_guard_claims_to_catch() {
    for form in THE_FOUR_FORMS {
        assert!(
            conditions_the_operating_system(form),
            "la guarda deberia cazar esta forma y no la caza: {form}"
        );
    }
}

#[test]
fn the_needles_leave_alone_what_is_not_an_operating_system_conditional() {
    for line in WHAT_MUST_NOT_TRIP_IT {
        assert!(
            !conditions_the_operating_system(line),
            "la guarda acusa a una linea que no decide por sistema operativo: {line}"
        );
    }
}
