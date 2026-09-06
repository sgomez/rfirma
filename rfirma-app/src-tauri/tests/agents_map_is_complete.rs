//! Guarda que comprueba que cada módulo del código está documentado en su `AGENTS.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Zona de código y su índice correspondiente.
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

/// Comprueba si el fichero es un test de la interfaz a excluir del mapa.
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

/// Módulos versionados de una zona devueltos por git.
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

/// Módulos de la zona ausentes del índice.
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
