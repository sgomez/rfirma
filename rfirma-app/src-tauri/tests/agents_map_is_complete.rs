//! Guarda de los mapas: cada módulo está en el `AGENTS.md` de su zona, y cada fila dice qué es el fichero, no cómo funciona (ADR-0017).

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

/// Las carpetas de primer nivel con mapa propio, con la ruta del suyo.
fn context_maps(root: &Path, zone: &Zone, modules: &[String]) -> Vec<(String, String)> {
    let mut folders: Vec<&str> = modules
        .iter()
        .filter_map(|relative| relative.split_once('/').map(|(folder, _)| folder))
        .collect();
    folders.sort_unstable();
    folders.dedup();
    folders
        .into_iter()
        .map(|folder| {
            (
                folder.to_owned(),
                format!("{}/{folder}/AGENTS.md", zone.root),
            )
        })
        .filter(|(_, map)| root.join(map).is_file())
        .collect()
}

/// Los índices de la zona: el raíz y uno por cada carpeta de primer nivel que tenga el suyo.
fn maps_of(root: &Path, zone: &Zone, modules: &[String]) -> Vec<(String, String)> {
    let mut maps = vec![(String::new(), read(root, zone.map))];
    for (folder, map) in context_maps(root, zone, modules) {
        maps.push((format!("{folder}/"), read(root, &map)));
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
             Anade una fila por cada uno: ruta y que es, en una frase. Sin tamanos: \
             los da `just outline`.",
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
    let maps = [root_map("| `memory/recents.rs` | Los diez recientes. |")];
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
    let maps = [root_map("| `mod.rs` | Algo. |")];
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
        root_map("| `site/` | El contexto de sede: ver `site/AGENTS.md`. |"),
        context_map("site", "| `adapters/tauri.rs` | Las ordenes de sede. |"),
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
        context_map("site", "| `adapters/tauri.rs` | Las ordenes. |"),
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

/// Lo que cabe en una fila de mapa: una frase que dice qué es el fichero.
const THE_LONGEST_ROW: usize = 300;

/// Prefijos de los identificadores de la especificación, que mueren con ella.
const SPEC_CITATIONS: [&str; 4] = ["ID-", "TD-", "RD-", "RT-"];

/// Todos los mapas del repositorio, por su ruta.
fn every_map(root: &Path) -> Vec<(String, String)> {
    let mut maps = Vec::new();
    for zone in &ZONES {
        maps.push((zone.map.to_owned(), read(root, zone.map)));
        let modules = tracked_modules(root, zone);
        for (_, map) in context_maps(root, zone, &modules) {
            let content = read(root, &map);
            maps.push((map, content));
        }
    }
    maps
}

/// La cabecera de la tabla que reparte los módulos de una zona.
const THE_TABLE_OF_MODULES: &str = "| Módulo | Qué es |";

/// Las filas de la tabla de módulos, que es la única que describe ficheros.
fn rows_of(map: &str) -> Vec<&str> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in map.lines() {
        if line.starts_with(THE_TABLE_OF_MODULES) {
            inside = true;
        } else if !line.starts_with('|') {
            inside = false;
        } else if inside && line.contains('`') {
            rows.push(line);
        }
    }
    rows
}

/// Comprueba si el texto cita un identificador de la especificación o un número de issue.
fn cites_something_that_dies(row: &str) -> bool {
    let dies_after = |prefix: &str| {
        row.match_indices(prefix)
            .any(|(at, _)| row[at + prefix.len()..].starts_with(|c: char| c.is_ascii_digit()))
    };
    let is_an_issue = row.match_indices('#').any(|(at, _)| {
        let before_it_starts = at == 0 || matches!(&row[at - 1..at], " " | "(");
        before_it_starts && row[at + 1..].starts_with(|c: char| c.is_ascii_digit())
    });

    SPEC_CITATIONS.iter().any(|prefix| dies_after(prefix)) || is_an_issue
}

/// Lo que sobra de una fila, si sobra algo.
fn what_is_wrong_with(row: &str) -> Option<String> {
    if cites_something_that_dies(row) {
        return Some(
            "cita un ID-NN o un numero de issue, que mueren antes que el codigo".to_owned(),
        );
    }
    let length = row.chars().count();
    if length > THE_LONGEST_ROW {
        return Some(format!(
            "son {length} caracteres: a partir de {THE_LONGEST_ROW} ya no dice que es, cuenta como funciona"
        ));
    }
    None
}

#[test]
fn a_map_row_says_what_the_file_is_and_stops_there() {
    let root = repository_root();
    let mut wrong = Vec::new();

    for (path, map) in every_map(&root) {
        for row in rows_of(&map) {
            if let Some(reason) = what_is_wrong_with(row) {
                let start: String = row.chars().take(60).collect();
                wrong.push(format!("{path}\n  {start}…\n  {reason}"));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "un mapa dice que es cada fichero para que un agente sepa cual abrir; el como lo dice el \
         codigo, que no se desincroniza, y el porque un ADR:\n\n{}",
        wrong.join("\n\n")
    );
}

#[test]
fn only_the_table_of_modules_is_read() {
    let map = format!(
        "{THE_TABLE_OF_MODULES}\n|---|---|\n| `a.rs` | Algo. |\n\n\
         | Bloque | Qué decide |\n|---|---|\n| L1-26 (`entriesOf`) | Otra cosa. |\n"
    );

    assert_eq!(
        rows_of(&map),
        vec!["| `a.rs` | Algo. |"],
        "el indice de comentarios de bloque de i18n no es un mapa de modulos"
    );
}

#[test]
fn a_row_that_explains_how_it_works_is_caught() {
    let short =
        "| `domain/trust.rs` | Las reglas de la CA local: cuando se instala y cuando se solapa. |";
    assert_eq!(what_is_wrong_with(short), None);

    let long = format!("| `a.rs` | {} |", "y ademas ".repeat(40));
    assert!(what_is_wrong_with(&long).is_some());
}

#[test]
fn a_row_that_cites_the_spec_or_an_issue_is_caught() {
    assert!(cites_something_that_dies(
        "| `a.rs` | Lo que sea (ID-215). |"
    ));
    assert!(cites_something_that_dies("| `a.rs` | Lo que sea (TD-9). |"));
    assert!(cites_something_that_dies(
        "| `a.rs` | Lo que sea, del #453. |"
    ));
    assert!(!cites_something_that_dies(
        "| `a.rs` | Lo que sea (ADR-0017). |"
    ));
    assert!(!cites_something_that_dies(
        "| `a.rs` | El identificador RD del formulario. |"
    ));
    assert!(
        !cites_something_that_dies("| `a.rs` | El modulo PKCS#11 del sistema. |"),
        "PKCS#11 no es un numero de issue"
    );
}
