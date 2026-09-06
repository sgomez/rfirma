//! Guarda que comprueba que cada módulo del código está en el `AGENTS.md` de su zona o en el de su contexto (RD-10).

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

/// Los índices de la zona: el raíz y uno por cada carpeta de primer nivel que tenga el suyo.
fn maps_of(root: &Path, zone: &Zone, modules: &[String]) -> Vec<(String, String)> {
    let mut maps = vec![(String::new(), read(root, zone.map))];
    let mut folders: Vec<&str> = modules
        .iter()
        .filter_map(|relative| relative.split_once('/').map(|(folder, _)| folder))
        .collect();
    folders.sort_unstable();
    folders.dedup();
    for folder in folders {
        let map = format!("{}/{folder}/AGENTS.md", zone.root);
        if root.join(&map).is_file() {
            maps.push((format!("{folder}/"), read(root, &map)));
        }
    }
    maps
}

fn read(root: &Path, map: &str) -> String {
    fs::read_to_string(root.join(map))
        .unwrap_or_else(|error| panic!("deberia leerse {map}: {error}"))
}

/// Comprueba si algún índice nombra al módulo: el raíz por su ruta entera, el de su contexto por la ruta desde ahí.
fn is_named(relative: &str, maps: &[(String, String)]) -> bool {
    maps.iter().any(|(prefix, map)| {
        relative
            .strip_prefix(prefix.as_str())
            .is_some_and(|inside| map.contains(&format!("`{inside}`")))
    })
}

/// Módulos de la zona ausentes de todos sus índices.
fn absent_from(maps: &[(String, String)], modules: &[String]) -> Vec<String> {
    modules
        .iter()
        .filter(|relative| !is_named(relative, maps))
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

        let maps = maps_of(&root, zone, &modules);

        let missing = absent_from(&maps, &modules);
        assert!(
            missing.is_empty(),
            "{} es lo que un agente lee en vez de explorar {}, y ni el ni los indices por \
             contexto nombran estos modulos:\n{}\n\
             Anade una fila por cada uno: ruta, tamano y que es, en una frase.",
            zone.map,
            zone.root,
            missing.join("\n")
        );
    }
}

fn root_map(map: &str) -> (String, String) {
    (String::new(), map.to_owned())
}

fn context_map(context: &str, map: &str) -> (String, String) {
    (format!("{context}/"), map.to_owned())
}

#[test]
fn a_map_that_forgets_a_module_is_caught() {
    let maps = [root_map(
        "| `memory/recents.rs` | 406 | Los diez recientes. |",
    )];
    let modules = [
        "memory/recents.rs".to_owned(),
        "memory/brand_new.rs".to_owned(),
    ];

    assert_eq!(
        absent_from(&maps, &modules),
        vec!["memory/brand_new.rs".to_owned()],
        "la guarda tiene que ver el modulo que falta y solo ese"
    );
}

#[test]
fn a_bare_file_name_does_not_count_as_naming_the_module() {
    let maps = [root_map("| `mod.rs` | 406 | Algo. |")];
    let modules = ["memory/mod.rs".to_owned()];

    assert_eq!(
        absent_from(&maps, &modules),
        modules.to_vec(),
        "nombrar el fichero suelto no vale: hay seis `mod.rs` y se taparian entre ellos"
    );
}

#[test]
fn a_module_named_in_the_map_of_its_context_is_not_missing() {
    let maps = [
        root_map("| `site/` | — | El contexto de sede: ver `site/AGENTS.md`. |"),
        context_map(
            "site",
            "| `adapters/tauri.rs` | 80 | Las ordenes de sede. |",
        ),
    ];
    let modules = [
        "site/adapters/tauri.rs".to_owned(),
        "site/domain/errand.rs".to_owned(),
        "memory/recents.rs".to_owned(),
    ];

    assert_eq!(
        absent_from(&maps, &modules),
        vec![
            "site/domain/errand.rs".to_owned(),
            "memory/recents.rs".to_owned()
        ],
        "el mapa del contexto nombra por la ruta desde su carpeta, y no cubre a otros"
    );
}

#[test]
fn a_context_map_does_not_name_a_module_of_another_context() {
    let maps = [
        root_map(""),
        context_map("site", "| `adapters/tauri.rs` | 80 | Las ordenes. |"),
    ];
    let modules = ["identity/adapters/tauri.rs".to_owned()];

    assert_eq!(absent_from(&maps, &modules), modules.to_vec());
}

#[test]
fn the_tests_of_the_window_are_not_asked_of_the_map() {
    assert!(is_a_test_file("signing/flow.test.ts"));
    assert!(is_a_test_file("App.test.tsx"));
    assert!(!is_a_test_file("signing/flow.ts"));
    assert!(!is_a_test_file("testing/render.tsx"));
}
