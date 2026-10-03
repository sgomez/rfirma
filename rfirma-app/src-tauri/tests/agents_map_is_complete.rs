//! Guarda de los índices: en el backend cada módulo abre con una cabecera `//!` que dice qué es, y en la interfaz cada módulo está en el `AGENTS.md` de su zona (ADR-0017).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Dónde se dice qué es cada módulo de una zona.
enum Naming {
    /// En su primera línea `//!`, que `just outline <directorio>/` junta en un índice.
    Header,
    /// En una fila del mapa de la zona o del de su contexto.
    MapRow,
}

/// Zona de código y su índice correspondiente.
struct Zone {
    /// Raíz de la zona, relativa a la raíz del repositorio.
    root: &'static str,
    /// El mapa raíz de la zona.
    map: &'static str,
    /// Extensiones que cuentan como módulo.
    extensions: &'static [&'static str],
    /// Dónde se dice qué es cada módulo.
    naming: Naming,
}

const ZONES: [Zone; 2] = [
    Zone {
        root: "rfirma-app/src-tauri/src",
        map: "rfirma-app/src-tauri/src/AGENTS.md",
        extensions: &["rs"],
        naming: Naming::Header,
    },
    Zone {
        root: "rfirma-app/src",
        map: "rfirma-app/src/AGENTS.md",
        extensions: &["ts", "tsx"],
        naming: Naming::MapRow,
    },
];

/// Comprueba si el fichero es de pruebas, al que no se le pide ni cabecera ni fila; `scripts/outline.sh` usa el mismo criterio.
fn is_a_test_file(relative: &str) -> bool {
    let rooted = format!("/{relative}");
    let a_rust_test = relative.ends_with(".rs")
        && (rooted.ends_with("/tests.rs")
            || rooted.ends_with("_tests.rs")
            || rooted.contains("/tests/"));
    a_rust_test || relative.ends_with(".test.ts") || relative.ends_with(".test.tsx")
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

    let modules: Vec<String> = String::from_utf8(listing.stdout)
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
        .collect();
    assert!(
        modules.len() > 5,
        "el listado no ha encontrado el codigo de {}: {} ficheros",
        zone.root,
        modules.len()
    );
    modules
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

fn read(root: &Path, path: &str) -> String {
    fs::read_to_string(root.join(path))
        .unwrap_or_else(|error| panic!("deberia leerse {path}: {error}"))
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

    for zone in ZONES
        .iter()
        .filter(|zone| matches!(zone.naming, Naming::MapRow))
    {
        let modules = tracked_modules(&root, zone);
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

/// Lo que sobra o falta en la cabecera con la que abre un módulo, si algo.
fn what_is_wrong_with_the_header(source: &str) -> Option<String> {
    let mut lines = source.lines();
    let Some(header) = lines
        .next()
        .and_then(|first| first.strip_prefix("//!"))
        .map(str::trim)
        .filter(|header| !header.is_empty())
    else {
        return Some("no abre con una linea `//!` que diga que es".to_owned());
    };
    let spills_over = lines
        .next()
        .and_then(|second| second.strip_prefix("//!"))
        .is_some_and(|rest| !rest.trim().is_empty());
    if spills_over {
        return Some(
            "la cabecera sigue en la segunda linea: una frase, en una sola linea `//!`".to_owned(),
        );
    }
    what_is_wrong_with(header)
}

#[test]
fn every_backend_module_opens_with_a_header_that_says_what_it_is() {
    let root = repository_root();

    for zone in ZONES
        .iter()
        .filter(|zone| matches!(zone.naming, Naming::Header))
    {
        let wrong: Vec<String> = tracked_modules(&root, zone)
            .iter()
            .filter_map(|relative| {
                let source = read(&root, &format!("{}/{relative}", zone.root));
                what_is_wrong_with_the_header(&source)
                    .map(|reason| format!("{relative}\n  {reason}"))
            })
            .collect();
        assert!(
            wrong.is_empty(),
            "en {} el indice de modulos lo da `just outline <directorio>/` con la primera linea \
             `//!` de cada uno: una frase que dice que es el modulo y, si ayuda, que no es; el \
             como lo dice el codigo y el porque un ADR:\n\n{}",
            zone.root,
            wrong.join("\n\n")
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
    let maps = [root_map("| `signing/flow.ts` | El flujo de firma. |")];
    let modules = [
        "signing/flow.ts".to_owned(),
        "signing/brand_new.ts".to_owned(),
    ];

    assert_eq!(
        absent_from(&maps, &modules),
        vec!["signing/brand_new.ts".to_owned()],
        "la guarda tiene que ver el modulo que falta y solo ese"
    );
}

#[test]
fn a_bare_file_name_does_not_count_as_naming_the_module() {
    let maps = [root_map("| `index.ts` | Algo. |")];
    let modules = ["signing/index.ts".to_owned()];

    assert_eq!(
        absent_from(&maps, &modules),
        modules.to_vec(),
        "nombrar el fichero suelto no vale: varios `index.ts` se taparian entre ellos"
    );
}

#[test]
fn a_module_named_in_the_map_of_its_context_is_not_missing() {
    let maps = [
        root_map("| `site/` | La ventana de sede: ver `site/AGENTS.md`. |"),
        context_map("site", "| `panel.tsx` | El panel de sede. |"),
    ];
    let modules = [
        "site/panel.tsx".to_owned(),
        "site/errand.ts".to_owned(),
        "memory/recents.ts".to_owned(),
    ];

    assert_eq!(
        absent_from(&maps, &modules),
        vec!["site/errand.ts".to_owned(), "memory/recents.ts".to_owned()],
        "el mapa del contexto nombra por la ruta desde su carpeta, y no cubre a otros"
    );
}

#[test]
fn a_context_map_does_not_name_a_module_of_another_context() {
    let maps = [
        root_map(""),
        context_map("site", "| `panel.tsx` | El panel. |"),
    ];
    let modules = ["identity/panel.tsx".to_owned()];

    assert_eq!(absent_from(&maps, &modules), modules.to_vec());
}

#[test]
fn the_tests_of_the_window_are_not_asked_of_the_map() {
    assert!(is_a_test_file("signing/flow.test.ts"));
    assert!(is_a_test_file("App.test.tsx"));
    assert!(!is_a_test_file("signing/flow.ts"));
    assert!(!is_a_test_file("testing/render.tsx"));
    assert!(
        !is_a_test_file("tests/render.tsx"),
        "una carpeta `tests/` solo aparta ficheros de Rust"
    );
}

#[test]
fn the_tests_of_the_backend_are_not_asked_for_a_header() {
    assert!(is_a_test_file("site/adapters/codec/tests.rs"));
    assert!(is_a_test_file("site/application/errand/tests/support.rs"));
    assert!(is_a_test_file(
        "desktop/adapters/firefox_lock/windows_tests.rs"
    ));
    assert!(is_a_test_file("tests.rs"));
    assert!(!is_a_test_file("site/adapters/header_probe.rs"));
    assert!(!is_a_test_file("crossing/guards.rs"));
    assert!(!is_a_test_file("site/domain/protocol/testsuite.rs"));
}

#[test]
fn a_module_without_a_header_is_caught() {
    assert!(what_is_wrong_with_the_header("use std::fs;\n").is_some());
    assert!(
        what_is_wrong_with_the_header("// Sin consola.\n//! Llega tarde.\n").is_some(),
        "la cabecera es la primera linea, no una `//!` cualquiera"
    );
    assert!(what_is_wrong_with_the_header("//!\n//! Despues del hueco.\n").is_some());
    assert!(what_is_wrong_with_the_header("").is_some());
}

#[test]
fn a_header_is_one_line_and_the_prose_below_it_is_free() {
    assert!(what_is_wrong_with_the_header("//! Una frase\n//! partida en dos.\n").is_some());
    assert_eq!(
        what_is_wrong_with_the_header(
            "//! Las reglas puras de la CA local (ADR-0005).\n//!\n//! Prosa aparte.\n"
        ),
        None
    );
    assert_eq!(
        what_is_wrong_with_the_header("//! El canal local.\n\nuse std::fs;\n"),
        None
    );
}

#[test]
fn a_header_that_cites_the_spec_or_explains_how_it_works_is_caught() {
    assert!(what_is_wrong_with_the_header("//! Lo que ya no compila (#439, #441).\n").is_some());
    assert!(what_is_wrong_with_the_header("//! Lo que sea (ID-215).\n").is_some());
    let long = format!("//! {}\n", "y ademas ".repeat(40));
    assert!(what_is_wrong_with_the_header(&long).is_some());
    assert_eq!(
        what_is_wrong_with_the_header("//! El modulo PKCS#11 del sistema (ADR-0022).\n"),
        None
    );
}

/// Lo que cabe en una fila de mapa o en una cabecera: una frase que dice qué es el fichero.
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

/// Lo que sobra de una fila o de una cabecera, si sobra algo.
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
        "{THE_TABLE_OF_MODULES}\n|---|---|\n| `a.ts` | Algo. |\n\n\
         | Bloque | Qué decide |\n|---|---|\n| L1-26 (`entriesOf`) | Otra cosa. |\n"
    );

    assert_eq!(
        rows_of(&map),
        vec!["| `a.ts` | Algo. |"],
        "el indice de comentarios de bloque de i18n no es un mapa de modulos"
    );
}

#[test]
fn a_row_that_explains_how_it_works_is_caught() {
    let short =
        "| `signing/flow.ts` | El flujo de firma: cuando se pide el PIN y cuando se cancela. |";
    assert_eq!(what_is_wrong_with(short), None);

    let long = format!("| `a.ts` | {} |", "y ademas ".repeat(40));
    assert!(what_is_wrong_with(&long).is_some());
}

#[test]
fn a_row_that_cites_the_spec_or_an_issue_is_caught() {
    assert!(cites_something_that_dies(
        "| `a.ts` | Lo que sea (ID-215). |"
    ));
    assert!(cites_something_that_dies("| `a.ts` | Lo que sea (TD-9). |"));
    assert!(cites_something_that_dies(
        "| `a.ts` | Lo que sea, del #453. |"
    ));
    assert!(!cites_something_that_dies(
        "| `a.ts` | Lo que sea (ADR-0017). |"
    ));
    assert!(!cites_something_that_dies(
        "| `a.ts` | El identificador RD del formulario. |"
    ));
    assert!(
        !cites_something_that_dies("| `a.ts` | El modulo PKCS#11 del sistema. |"),
        "PKCS#11 no es un numero de issue"
    );
}
