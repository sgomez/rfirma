//! `paths.rs` es el **único** fichero del repositorio con un condicional de
//! sistema operativo (ID-35, ADR-0010), y esto lo comprueba.
//!
//! **Grada A**: lee ficheros del repositorio y nada más. Sin token, sin
//! librería nativa y sin red.
//!
//! La regla no es un gusto estético. Si el `cfg!` se reparte, «añadir Windows»
//! deja de ser tocar un fichero y pasa a ser buscar por todo el árbol qué se
//! olvidó; y cada `cfg!` fuera de `paths.rs` es código que **no compila** en
//! los otros dos sistemas y que nadie ve fallar hasta que alguien construye
//! ahí. Una prueba es la única forma de que la regla siga viva cuando el
//! ADR-0010 lleve dos años escrito.
//!
//! Si esta prueba te ha puesto el PR en rojo: no la relajes. Mueve la decisión
//! a `src/paths.rs`, devuelve desde ahí un valor —una ruta, una variante de
//! [`rfirma_lib::paths::Platform`]— y deja que el resto del código sea código
//! normal que recibe ese valor.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// El único fichero autorizado, relativo a la raíz del repositorio.
const THE_ONLY_SITE: &str = "rfirma-app/src-tauri/src/paths.rs";

/// Esta misma prueba, que habla de `cfg!` sin usarlo y no puede acusarse a sí
/// misma.
const THIS_TEST: &str = "rfirma-app/src-tauri/tests/single_cfg_os_site.rs";

/// Lo que se busca, partido para que aparecer aquí no cuente como aparición.
/// Cubre las dos formas: el atributo `#[cfg(...)]` y la macro `cfg!(...)`.
const NEEDLES: [&str; 4] = ["target_", "os = \"", "cfg(unix", "cfg(windows"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Los `.rs` **versionados**, preguntándoselo a git.
///
/// Se le pregunta a git y no se recorre el árbol a mano por dos razones: los
/// directorios de construcción (`target/`) traen Rust generado que no es
/// nuestro, y un repositorio con árboles de trabajo enlazados dentro —como los
/// que usan los agentes— tiene copias enteras del código en otras ramas, que no
/// son este PR.
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

/// Un condicional de sistema operativo es `target_os = "…"`, `target_family =
/// "…"`, `cfg(unix)` o `cfg(windows)`. Se buscan las piezas por separado y se
/// exige que estén en la **misma línea**, para no confundir una línea de prosa
/// que mencione una de ellas con código.
fn conditions_the_operating_system(line: &str) -> bool {
    (line.contains(NEEDLES[0]) && line.contains(NEEDLES[1]))
        || line.contains(NEEDLES[2])
        || line.contains(NEEDLES[3])
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
