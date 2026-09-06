//! Guardas de verificación para las órdenes y tipos del adaptador de Tauri (ADR-0011).

use std::collections::BTreeSet;
use std::path::Path;

/// Ficheros del módulo con su código fuente.
const SOURCES: [(&str, &str); 7] = [
    ("mod.rs", include_str!("mod.rs")),
    ("failure.rs", include_str!("failure.rs")),
    ("orders.rs", include_str!("orders.rs")),
    ("rubric.rs", include_str!("rubric.rs")),
    ("site_window.rs", include_str!("site_window.rs")),
    ("views.rs", include_str!("views.rs")),
    ("views_site.rs", include_str!("views_site.rs")),
];

/// Fichero excluido de las comprobaciones de tipos.
const THIS_FILE: &str = "guards.rs";

/// Obtiene el código fuente de un fichero del módulo por su nombre.
fn source_of(file: &str) -> &'static str {
    SOURCES
        .iter()
        .find(|(name, _)| *name == file)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("«{file}» tiene que estar en la lista del modulo"))
}

/// El código fuente de un fichero de `SOURCES`, ya sin módulo de pruebas: vive en su
/// `tests.rs` hermano y `include_str!` no lo trae.
fn production_half(source: &str) -> &str {
    source
}

/// Aplana atributos de múltiples líneas a una sola línea.
fn attributes_on_one_line(source: &str) -> String {
    let mut joined = String::new();
    let mut open = 0i32;
    for line in source.lines() {
        let trimmed = line.trim_start();
        let starts_an_attribute = open == 0 && trimmed.starts_with("#[");
        if starts_an_attribute || open > 0 {
            if open == 0 {
                joined.push_str(line);
            } else {
                joined.push_str(trimmed);
            }
            open += line.matches('[').count() as i32 - line.matches(']').count() as i32;
            if open <= 0 {
                open = 0;
                joined.push('\n');
            }
            continue;
        }
        joined.push_str(line);
        joined.push('\n');
    }
    joined
}

/// Cuenta cuántas derivaciones de Serialize contiene el código aplanado.
fn serialising_derives(flattened: &str) -> usize {
    flattened
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("#[derive(") && line.contains("Serialize"))
        .count()
}

/// Tipo de salida serializable descubierto.
struct Output<'a> {
    file: &'a str,
    name: String,
}

/// Descubre todos los tipos de salida serializables declarados en el módulo.
fn outputs() -> Vec<Output<'static>> {
    let mut found = Vec::new();
    for (file, source) in SOURCES {
        let flattened = attributes_on_one_line(production_half(source));
        let mut serialisable = false;
        let mut open: Option<String> = None;
        for line in flattened.lines() {
            let trimmed = line.trim_start();
            if let Some(name) = open.as_mut() {
                if line == "}" {
                    found.push(Output {
                        file,
                        name: std::mem::take(name),
                    });
                    open = None;
                }
                continue;
            }
            if trimmed.starts_with("#[derive(") {
                serialisable = trimmed.contains("Serialize");
                continue;
            }
            if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("///") {
                continue;
            }
            let declaration = trimmed
                .strip_prefix("pub struct ")
                .or_else(|| trimmed.strip_prefix("pub enum "));
            if let Some(rest) = declaration {
                if serialisable {
                    let name = rest
                        .split(|letter: char| !letter.is_alphanumeric() && letter != '_')
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    open = Some(name);
                }
            }
            serialisable = false;
        }
    }
    found
}

/// Tipos de salida que no contienen información procedente de un documento.
const OUTPUTS_WITH_NO_DOCUMENT_BEHIND: [&str; 15] = [
    "StatusView",
    "CertificateView",
    "PlacementView",
    "RubricView",
    "SecretView",
    "NewVersionView",
    "UrlHandlersView",
    "UrlHandlerView",
    "SiteErrandView",
    "SiteStageView",
    "SignatureRoundView",
    "NoChannelView",
    "SiteOutcomeView",
    "RefusalSituationView",
    "NoCertificateView",
];

/// Ruta de prueba simulando un enlace concedido por el portal.
const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

/// Segunda ruta de prueba concedida por el portal.
const ANOTHER_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/segundo.pdf";

/// Registro de un tipo y su serialización JSON.
struct Crossing {
    name: &'static str,
    json: serde_json::Value,
}

impl Crossing {
    /// Construye un cruce a partir de un valor serializable.
    fn of(name: &'static str, value: &impl serde::Serialize) -> Self {
        Self {
            name,
            json: serde_json::to_value(value).expect("un tipo de salida tiene que serializar"),
        }
    }
}

/// Comprueba si una cadena contiene una ruta al portal de documentos.
fn is_a_portal_path(text: &str) -> bool {
    text.match_indices("/run/user/").any(|(at, _)| {
        text[at..]
            .split('/')
            .nth(4)
            .is_some_and(|segment| segment == "doc")
    })
}

/// Busca recursivamente rutas del portal dentro de un valor JSON.
fn the_portal_path_inside(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => is_a_portal_path(text).then(|| text.clone()),
        serde_json::Value::Array(items) => items.iter().find_map(the_portal_path_inside),
        serde_json::Value::Object(fields) => fields.values().find_map(the_portal_path_inside),
        _ => None,
    }
}

/// Genera todas las salidas producidas a partir de un documento del portal.
fn crossings_from_a_portal_document() -> Vec<Crossing> {
    use crate::app::fixtures::a_memory;
    use crate::app::{configuration, documents, recents};
    use crate::destination::{CheckedFolder, DestinationFolder, PortalDocument};
    use crate::memory::{Badge, Configuration, OpenedDocuments, RecentDocument, State};

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let document = PortalDocument::opened(A_PORTAL_HANDLE);
    let configuration = Configuration {
        destination: Some(DestinationFolder::at(
            Path::new(A_PORTAL_HANDLE)
                .parent()
                .expect("la concesion tiene directorio"),
        )),
        remember_activity: true,
        ..Configuration::default()
    };

    let opened_view = documents::note_opened(
        &memory,
        &configuration,
        &opened,
        std::path::PathBuf::from(A_PORTAL_HANDLE),
    );
    let failure = documents::bytes_of(&opened, &opened_view.id)
        .expect_err("el enlace del portal no existe fuera del sandbox");
    let dropped = documents::dropped_document(
        &[
            std::path::PathBuf::from(A_PORTAL_HANDLE),
            std::path::PathBuf::from(ANOTHER_PORTAL_HANDLE),
        ],
        &opened,
    )
    .expect("se ha soltado un fichero");
    let folder = CheckedFolder::at(home.path()).expect("el temporal esta ahi");
    let refused_rubric = crate::app::rubric::choose(
        &crate::rubric::RubricStore::at(home.path().join("rubric.jpg")),
        tauri_plugin_dialog::FilePath::Path(std::path::PathBuf::from(A_PORTAL_HANDLE)),
    )
    .expect_err("el enlace del portal no existe fuera del sandbox");

    let mut crossings = vec![
        Crossing::of("OpenedDocumentView", &opened_view),
        Crossing::of("Failure", &failure),
        Crossing::of("DroppedDocumentView", &dropped),
        Crossing::of(
            "DestinationView",
            &documents::where_it_lands(&configuration, home.path(), &document),
        ),
        Crossing::of(
            "SignedDocumentView",
            &documents::told_as(document.reading_path(), &folder, 42),
        ),
        Crossing::of(
            "ConfigurationView",
            &configuration::shown(&configuration, home.path()),
        ),
        Crossing::of(
            "RubricChoiceView",
            &crate::commands::RubricChoiceView::refused(&refused_rubric),
        ),
    ];

    let entry: RecentDocument = serde_json::from_value(serde_json::json!({
        "path": A_PORTAL_HANDLE,
        "name": "contrato.pdf",
        "badge": serde_json::to_value(Badge::Unsigned).expect("la insignia serializa"),
        "modified": 1_700_000_000_u64,
        "last_used": 1_700_000_100_u64,
    }))
    .expect("la fila del fichero de estado deberia leerse");
    let mut state = State::default();
    state.recents.record(entry);
    memory
        .remember_state(&configuration, &state)
        .expect("deberia guardarse el estado");
    for row in recents::listed_rows(&memory, &opened) {
        crossings.push(Crossing::of("RecentDocumentView", &row));
    }

    crossings
}

#[test]
fn the_portal_path_never_crosses_to_the_window() {
    let crossings = crossings_from_a_portal_document();

    for crossing in &crossings {
        assert!(
            the_portal_path_inside(&crossing.json).is_none(),
            "«{}» ha cruzado con una ruta del portal dentro: {}",
            crossing.name,
            crossing.json
        );
    }
}

#[test]
fn every_output_type_is_either_built_from_a_document_or_declared_without_one() {
    let outputs = outputs();
    let declared: usize = SOURCES
        .iter()
        .map(|(_, source)| serialising_derives(&attributes_on_one_line(production_half(source))))
        .sum();
    assert_eq!(
        outputs.len(),
        declared,
        "el modulo declara {declared} tipos serializables y el descubrimiento ha encontrado {}: \
         uno se esta escapando de la guarda",
        outputs.len()
    );
    assert!(
        outputs.len() >= 12,
        "los tipos de salida no se han encontrado: {}",
        outputs.len()
    );

    let built: BTreeSet<&str> = crossings_from_a_portal_document()
        .iter()
        .map(|crossing| crossing.name)
        .collect();
    let without: BTreeSet<&str> = OUTPUTS_WITH_NO_DOCUMENT_BEHIND.into_iter().collect();

    for output in &outputs {
        assert!(
            built.contains(output.name.as_str()) || without.contains(output.name.as_str()),
            "«{}» ({}) no se construye desde un documento del portal ni esta declarado como \
             tipo sin documento detras: la guarda de rutas no lo mira",
            output.name,
            output.file
        );
    }

    let known: BTreeSet<&str> = outputs.iter().map(|output| output.name.as_str()).collect();
    for name in built.iter().chain(without.iter()) {
        assert!(
            known.contains(name),
            "«{name}» ya no es un tipo de salida del modulo: sobra de la guarda"
        );
    }
}

#[test]
fn a_portal_path_buried_inside_a_field_is_a_leak() {
    let value = serde_json::json!({
        "name": "contrato.pdf",
        "failure": {
            "situation": "documentUnreadable",
            "detail": format!("no se ha podido leer {A_PORTAL_HANDLE}: no such file"),
        },
        "rows": [{ "id": "0f1e", "note": A_PORTAL_HANDLE }],
    });

    assert!(
        the_portal_path_inside(&value).is_some(),
        "una ruta del portal dentro de un campo es una fuga, este donde este"
    );
}

#[test]
fn a_home_path_inside_a_field_is_not_a_leak() {
    let value = serde_json::json!({
        "name": "contrato.pdf",
        "path": "/home/quien/Contratos/contrato.pdf",
        "socket": "/run/user/1000/keyring/pkcs11",
    });

    assert_eq!(
        the_portal_path_inside(&value),
        None,
        "solo la ruta del portal es una fuga: la real se enseña"
    );
}

/// Lista los ficheros Rust dentro de un directorio y sus subdirectorios.
fn rust_files_under(directory: &Path, prefix: &str) -> Vec<String> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(directory)
        .expect("el modulo de ordenes tiene que estar donde dice el manifiesto");
    for entry in entries {
        let entry = entry.expect("deberia leerse la entrada");
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if entry.path().is_dir() {
            found.extend(rust_files_under(&entry.path(), &format!("{relative}/")));
        } else if name.ends_with(".rs") {
            found.push(relative);
        }
    }
    found
}

#[test]
fn the_list_of_files_covers_the_whole_module() {
    let listed: BTreeSet<&str> = SOURCES.iter().map(|(file, _)| *file).collect();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    let present: BTreeSet<String> = rust_files_under(&directory, "")
        .into_iter()
        .filter(|name| name != THIS_FILE)
        .filter(|name| name != "tests.rs" && !name.ends_with("/tests.rs"))
        .collect();

    let missing: Vec<&String> = present
        .iter()
        .filter(|name| !listed.contains(name.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "estos ficheros del modulo no los recorre la guarda de rutas: {missing:?}"
    );
}

#[test]
fn the_list_of_commands_is_closed_and_this_is_how_long_it_is() {
    let orders: usize = SOURCES
        .iter()
        .map(|(_, source)| production_half(source).matches("#[tauri::command").count())
        .sum();

    assert_eq!(orders, 37, "la lista de ordenes es cerrada a proposito");
}

/// Extrae las declaraciones de órdenes Tauri del código fuente.
fn commands_of(source: &str) -> Vec<(&str, String, &str)> {
    let marker = "#[tauri::command";
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(marker) {
        rest = &rest[start..];
        let end = rest[marker.len()..]
            .find(marker)
            .map_or(rest.len(), |offset| offset + marker.len());
        let block = &rest[..end];
        let attribute = block.lines().next().unwrap_or_default();
        let name = block
            .split_once("pub fn ")
            .and_then(|(_, after)| after.split_once('('))
            .map(|(name, _)| name.trim().to_owned())
            .unwrap_or_else(|| "sin nombre".to_owned());
        found.push((attribute, name, block));
        rest = &rest[end..];
    }
    found
}

#[test]
fn every_command_that_touches_the_portal_runs_off_the_main_thread() {
    let source = production_half(source_of("mod.rs"));

    for command in [
        "pub fn open_document(",
        "pub fn read_document(",
        "pub fn open_signed_document(",
        "pub fn open_signed_folder(",
        "pub fn check_for_new_version(",
    ] {
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        let before = &source[..declaration];
        assert!(
            before.ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }

    let commands = commands_of(source);
    assert!(
        commands.len() >= 14,
        "el troceado de ordenes ha encontrado {}: si no las ve todas, no vigila nada",
        commands.len()
    );
    for (attribute, name, block) in commands {
        if !block.contains("blocking_") {
            continue;
        }
        assert_eq!(
            attribute.trim(),
            "#[tauri::command(async)]",
            "«{name}» llama a un blocking_* de un plugin desde el hilo del bucle de \
             eventos: tiene que ser #[tauri::command(async)] o la ventana se clava sin error"
        );
    }
}

#[test]
fn every_command_of_the_site_errand_runs_off_the_main_thread() {
    let source = production_half(source_of("mod.rs"));

    const OF_THE_ERRAND: [&str; 6] = [
        "pub fn close_site_window(",
        "pub fn site_identify(",
        "pub fn site_decline(",
        "pub fn install_local_ca(",
        "pub fn site_install_certificate(",
        "pub fn site_look_again(",
    ];

    for command in OF_THE_ERRAND {
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        assert!(
            source[..declaration].ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }
}

#[test]
fn the_pin_is_taken_by_a_single_command() {
    let takers: usize = SOURCES
        .iter()
        .map(|(_, source)| production_half(source).matches("pin: String").count())
        .sum();

    assert_eq!(takers, 1, "el PIN entra por una sola orden");
}

#[test]
fn a_derive_broken_across_lines_is_still_seen() {
    let broken = "#[derive(\n    Clone,\n    Debug,\n    Serialize,\n)]\npub struct Leaky {\n}\n";

    let flattened = attributes_on_one_line(broken);

    assert_eq!(
        serialising_derives(&flattened),
        1,
        "un derive partido en varias lineas sigue siendo un tipo de salida"
    );
    assert!(
        flattened
            .lines()
            .next()
            .is_some_and(|line| line.starts_with("#[derive(") && line.contains("Serialize")),
        "el atributo tiene que quedar en una sola linea: {flattened}"
    );
}

#[test]
fn what_is_not_an_attribute_is_left_alone() {
    let source = "pub struct Plain {\n    name: String,\n}\n";

    assert_eq!(attributes_on_one_line(source), source);
}
