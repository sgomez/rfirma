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

/// Lo que se busca.
///
/// Dos familias, y hacen falta las dos porque se escriben distinto:
///
/// - `target_os` y `target_family` bastan por sí solos: fuera de un `cfg` no
///   aparecen en Rust, y buscarlos enteros evita depender de si alguien
///   escribió `target_os="x"` sin espacios alrededor del `=`.
/// - `unix` y `windows` son palabras corrientes, así que se exigen **abiertas
///   por un paréntesis** —`(unix`, `(windows`— y con `cfg` en la misma línea.
///   Buscar el paréntesis y no `cfg(` literal es lo que hace que caigan las
///   cuatro formas: el atributo `#[cfg(unix)]`, la macro `cfg!(unix)` —donde
///   el `!` se mete en medio—, el `cfg(not(unix))` y el `cfg(any(unix, …))`.
const NEEDLES: [&str; 5] = ["cfg", "target_os", "target_family", "(unix", "(windows"];

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
/// "…"`, o un `cfg` que nombre `unix` o `windows` de cualquiera de las formas.
/// Para estas dos últimas se exige que el `cfg` esté en la **misma línea**,
/// para no confundir una línea de prosa que las mencione con código.
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

/// Las cuatro formas que la guarda dice cazar, escritas enteras.
///
/// Pueden escribirse literales porque este fichero está en [`THIS_TEST`] y la
/// guarda no se acusa a sí misma. Si algún día se quita esa excepción, esto
/// hay que partirlo; mientras tanto, leerlas de verdad es lo que hace que la
/// prueba valga.
const THE_FOUR_FORMS: [&str; 6] = [
    r#"#[cfg(target_os = "windows")]"#,
    r#"#[cfg(target_family = "unix")]"#,
    "    if cfg!(unix) { uno() } else { otro() }",
    "#[cfg(not(unix))]",
    r#"#[cfg(any(unix, target_env = "musl"))]"#,
    "#[cfg(windows)]",
];

/// Prosa y condicionales que **no** son de sistema operativo. Ninguno puede
/// disparar la guarda, o el PR rojo lo daría cualquier fichero.
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
