//! Solo los ficheros de `AUTHORISED_SITES` llevan un condicional de sistema operativo (ADR-0010, ADR-0035).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Los ficheros autorizados, relativos a la raíz del repositorio.
const AUTHORISED_SITES: [&str; 28] = [
    "rfirma-app/src-tauri/build.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/channel.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/choice/tests.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/firefox_lock.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/installer.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/paths.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/paths/tests.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/process.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/registry.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/terminal/descriptor.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/terminal/tty.rs",
    "rfirma-app/src-tauri/src/desktop/adapters/titlebar.rs",
    "rfirma-app/src-tauri/src/desktop/application/invocation/tests.rs",
    "rfirma-app/src-tauri/src/documents/domain/dropped/tests.rs",
    "rfirma-app/src-tauri/src/documents/domain/recents/tests.rs",
    "rfirma-app/src-tauri/src/identity/adapters/mod.rs",
    "rfirma-app/src-tauri/src/identity/adapters/pkcs11/stores/tests.rs",
    "rfirma-app/src-tauri/src/identity/domain/protected_secret.rs",
    "rfirma-app/src-tauri/src/signing/adapters/gtk_prompter.rs",
    "rfirma-app/src-tauri/src/signing/application/cycle/tests/on_a_card.rs",
    "rfirma-app/src-tauri/src/site/adapters/channel/acceptor.rs",
    "rfirma-app/src-tauri/src/site/adapters/mod.rs",
    "rfirma-app/src-tauri/src/site/adapters/scratch.rs",
    "rfirma-app/src-tauri/src/startup_dialog.rs",
    "rfirma-app/src-tauri/tests/command_line_pin_fd.rs",
    "rfirma-app/src-tauri/tests/fake_card.rs",
    "rfirma-app/src-tauri/tests/native_fake_card.rs",
    "rfirma-app/src-tauri/tests/native_leak.rs",
];

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
fn only_the_authorised_files_know_the_operating_system() {
    let root = repository_root();
    let files = tracked_rust_files(&root);
    assert!(
        files.len() > 5,
        "el listado no ha encontrado el codigo: {} ficheros",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for relative in &files {
        if AUTHORISED_SITES.contains(&relative.as_str()) || relative == THIS_TEST {
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
        "el ADR-0035 pone TODO el conocimiento del sistema operativo en AUTHORISED_SITES, \
         y estas lineas lo sacan de ahi:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn every_authorised_file_is_there_and_really_carries_the_conditional() {
    for site in AUTHORISED_SITES {
        let contents = fs::read_to_string(repository_root().join(site))
            .unwrap_or_else(|error| panic!("{site} deberia existir: {error}"));

        assert!(
            contents.lines().any(conditions_the_operating_system),
            "{site} ya no decide por sistema operativo: sobra en AUTHORISED_SITES"
        );
    }
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
