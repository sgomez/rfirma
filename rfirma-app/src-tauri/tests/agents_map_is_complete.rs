//! Los mapas de `AGENTS.md` **no valen si mienten**, y mienten en cuanto
//! alguien añade un módulo y no los toca.
//!
//! **Grada A**: lee ficheros del repositorio y nada más. Sin token, sin
//! librería nativa y sin red.
//!
//! Cada zona del código tiene un índice —`rfirma-app/src-tauri/src/AGENTS.md`
//! y `rfirma-app/src/AGENTS.md`— que un agente lee **en lugar de** explorar el
//! árbol: le dice qué hay, dónde y cuánto pesa, para que abra un fichero en vez
//! de veinte. Medido en la sesión del #126, esa exploración a ciegas se llevó
//! el 58 % del contexto de un trabajador y escribir el parche el 3 %.
//!
//! Un índice al que le falta un módulo es peor que no tener índice: el agente
//! se fía, no encuentra lo que busca, y acaba haciendo el `find` y el `cat`
//! entero que el índice existía para evitarle. Y nadie se entera, porque quien
//! añade un módulo no tiene ninguna razón para leer el índice.
//!
//! De ahí esta guarda. Comprueba lo único que de verdad rompe un índice —que
//! falte un fichero—, no que la descripción siga siendo cierta ni que el número
//! de líneas esté al día: eso se degrada despacio y se arregla leyendo.
//!
//! Si esta prueba te ha puesto el PR en rojo: **añade la fila**. Una línea en
//! la tabla, con la ruta, el tamaño y qué es el módulo en una frase.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Una zona de código con su índice.
struct Zone {
    /// Raíz de la zona, relativa a la raíz del repositorio.
    root: &'static str,
    /// El índice que debe nombrar todo lo que hay dentro.
    map: &'static str,
    /// Extensiones que cuentan como módulo.
    extensions: &'static [&'static str],
}

const ZONES: [Zone; 2] = [
    Zone {
        root: "rfirma-app/src-tauri/src",
        map: "rfirma-app/src-tauri/src/AGENTS.md",
        extensions: &["rs"],
    },
    Zone {
        root: "rfirma-app/src",
        map: "rfirma-app/src/AGENTS.md",
        extensions: &["ts", "tsx"],
    },
];

/// Lo que el índice **no** tiene que nombrar.
///
/// Solo los tests de la interfaz, que viven en `*.test.ts(x)` al lado de su
/// módulo y a los que el propio índice manda no entrar. Los de Rust van dentro
/// del módulo, así que no hay nada que excluir por ese lado.
fn is_a_test_file(relative: &str) -> bool {
    relative.ends_with(".test.ts") || relative.ends_with(".test.tsx")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("src-tauri deberia colgar de la raiz del repositorio")
        .to_path_buf()
}

/// Los ficheros **versionados** de una zona, preguntándoselo a git.
///
/// Se le pregunta a git y no se recorre el árbol a mano por lo mismo que en
/// `single_cfg_os_site.rs`: `target/` y `node_modules/` traen código que no es
/// nuestro, y un repositorio con árboles de trabajo enlazados dentro —como los
/// que usan los agentes— tiene copias enteras del código en otras ramas.
fn tracked_modules(root: &Path, zone: &Zone) -> Vec<String> {
    let listing = Command::new("git")
        .args(["ls-files", "-z", zone.root])
        .current_dir(root)
        .output()
        .expect("git deberia estar: `just tools` lo exige");
    assert!(listing.status.success(), "git ls-files deberia funcionar");

    String::from_utf8(listing.stdout)
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
        .collect()
}

/// Los módulos de la zona que el índice no nombra.
///
/// Nombrar es aparecer con la **ruta relativa a la zona** —`memory/recents.rs`,
/// no `recents.rs`—, porque el nombre suelto se repite: hay seis `mod.rs`, y un
/// índice que solo dijera «mod.rs» daría por cubierto cualquier módulo nuevo.
fn absent_from(map: &str, modules: &[String]) -> Vec<String> {
    modules
        .iter()
        .filter(|relative| !map.contains(relative.as_str()))
        .cloned()
        .collect()
}

#[test]
fn every_module_is_named_in_the_map_of_its_zone() {
    let root = repository_root();

    for zone in &ZONES {
        let modules = tracked_modules(&root, zone);
        assert!(
            modules.len() > 5,
            "el listado no ha encontrado el codigo de {}: {} ficheros",
            zone.root,
            modules.len()
        );

        let map = fs::read_to_string(root.join(zone.map))
            .unwrap_or_else(|error| panic!("deberia leerse {}: {error}", zone.map));

        let missing = absent_from(&map, &modules);
        assert!(
            missing.is_empty(),
            "{} es lo que un agente lee en vez de explorar {}, y no nombra estos modulos:\n{}\n\
             Anade una fila por cada uno: ruta, tamano y que es, en una frase.",
            zone.map,
            zone.root,
            missing.join("\n")
        );
    }
}

#[test]
fn a_map_that_forgets_a_module_is_caught() {
    let map = "| `memory/recents.rs` | 406 | Los diez recientes. |";
    let modules = [
        "memory/recents.rs".to_owned(),
        "memory/brand_new.rs".to_owned(),
    ];

    assert_eq!(
        absent_from(map, &modules),
        vec!["memory/brand_new.rs".to_owned()],
        "la guarda tiene que ver el modulo que falta y solo ese"
    );
}

#[test]
fn a_bare_file_name_does_not_count_as_naming_the_module() {
    let map = "| `mod.rs` | 406 | Algo. |";
    let modules = ["memory/mod.rs".to_owned()];

    assert_eq!(
        absent_from(map, &modules),
        modules.to_vec(),
        "nombrar el fichero suelto no vale: hay seis `mod.rs` y se taparian entre ellos"
    );
}

#[test]
fn the_tests_of_the_window_are_not_asked_of_the_map() {
    assert!(is_a_test_file("signing/flow.test.ts"));
    assert!(is_a_test_file("App.test.tsx"));
    assert!(!is_a_test_file("signing/flow.ts"));
    assert!(!is_a_test_file("testing/render.tsx"));
}
