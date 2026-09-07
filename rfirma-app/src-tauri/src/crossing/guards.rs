//! Guardas de verificación para las órdenes y los tipos que cruzan a la ventana (ADR-0011): las órdenes se leen del fuente, los tipos del registro.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

use crate::crossing::{all_crossings, type_names_in, Crossing};

/// Fichero excluido de las comprobaciones de tipos.
const THIS_FILE: &str = "guards.rs";

/// Comprueba si la ruta, relativa a `src/`, es del adaptador de Tauri: en el `adapters/` de un contexto, un `tauri*`, `views*` u `orders*` (RD-02).
fn is_an_adapter(relative: &str) -> bool {
    let mut segments = relative.split('/');
    let name = relative.rsplit('/').next().unwrap_or_default();
    segments.nth(1) == Some("adapters")
        && ["tauri", "views", "orders"]
            .iter()
            .any(|stem| name.starts_with(stem))
}

/// Comprueba si el fichero es de producción: ni pruebas hermanas ni esta guarda.
fn is_production(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or_default();
    name != "tests.rs" && name != THIS_FILE
}

/// Ficheros de adaptador bajo `src/`, con su código, sin lista que mantener.
fn adapter_sources_under(src: &Path) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = rust_files_under(src, "")
        .into_iter()
        .filter(|relative| is_an_adapter(relative) && is_production(relative))
        .map(|relative| {
            let source = std::fs::read_to_string(src.join(&relative))
                .unwrap_or_else(|error| panic!("deberia leerse {relative}: {error}"));
            (relative, source)
        })
        .collect();
    found.sort();
    found
}

/// Los ficheros de adaptador de este crate, leídos una sola vez.
fn sources() -> &'static [(String, String)] {
    static SOURCES: OnceLock<Vec<(String, String)>> = OnceLock::new();
    SOURCES
        .get_or_init(|| adapter_sources_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")))
}

/// Obtiene el código fuente de un fichero por su ruta relativa a `src/`.
fn source_of(file: &str) -> &'static str {
    sources()
        .iter()
        .find(|(name, _)| name == file)
        .map(|(_, source)| source.as_str())
        .unwrap_or_else(|| panic!("«{file}» tiene que ser un adaptador de src/"))
}

/// El código fuente de un fichero, ya sin módulo de pruebas: vive en su `tests.rs` hermano.
fn production_half(source: &str) -> &str {
    source
}

/// Los tipos del registro que cruzan de verdad: los declarados en una prueba hermana no salen a ninguna ventana.
fn registry() -> Vec<&'static Crossing> {
    all_crossings()
        .into_iter()
        .filter(|crossing| !crossing.file.ends_with("/tests.rs"))
        .collect()
}

/// Los tipos de salida: lo que el registro dice que deriva `Serialize` y no es prestado.
fn outputs() -> Vec<&'static Crossing> {
    registry()
        .into_iter()
        .filter(|crossing| crossing.lent_from.is_none() && crossing.serialises())
        .collect()
}

/// Lo que la biblioteca estándar y Tauri ponen en una firma y no hay que explicar.
const NOT_OURS: [&str; 11] = [
    "Option",
    "Vec",
    "String",
    "Box",
    "Result",
    "HashMap",
    "BTreeMap",
    "BTreeSet",
    "State",
    "AppHandle",
    "Response",
];

/// Los tipos que nombra la firma de una orden, sin el estado inyectado ni lo que no es nuestro.
fn types_named_by(signature: &str) -> Vec<&str> {
    let mut rest = signature;
    let mut words = Vec::new();
    while let Some(at) = rest.find("State<") {
        words.extend(type_names_in(&rest[..at]));
        let after = &rest[at + "State<".len()..];
        let close = after.find('>').expect("State<...> se cierra");
        rest = &after[close + 1..];
    }
    words.extend(type_names_in(rest));
    words.retain(|word| !NOT_OURS.contains(word));
    words.sort_unstable();
    words.dedup();
    words
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

/// Un tipo de salida ya serializado, con su nombre.
struct Serialised {
    name: &'static str,
    json: serde_json::Value,
}

impl Serialised {
    /// Serializa un valor de salida bajo su nombre.
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
fn crossings_from_a_portal_document() -> Vec<Serialised> {
    use crate::crossing::Failure;
    use crate::documents::adapters::views::{
        DestinationView, DroppedDocumentView, OpenedDocumentView, RecentDocumentView,
        SignedDocumentView,
    };
    use crate::documents::application::documents::OpenedDocuments;
    use crate::documents::application::{documents, recents};
    use crate::documents::domain::destination::{CheckedFolder, DestinationFolder};
    use crate::documents::domain::document::Document;
    use crate::documents::domain::recents::Badge;
    use crate::documents::domain::recents::RecentDocument;
    use crate::signing::adapters::state::State;
    use crate::signing::adapters::views::ConfigurationView;
    use crate::signing::application::configuration;
    use crate::signing::application::configuration_memory::Configuration;
    use crate::signing::application::tests::a_memory;

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let document = Document::opened(A_PORTAL_HANDLE);
    let chosen = DestinationFolder::at(
        Path::new(A_PORTAL_HANDLE)
            .parent()
            .expect("la concesion tiene directorio"),
    );
    let configuration = Configuration {
        destination: Some(chosen.clone()),
        remember_activity: true,
        ..Configuration::default()
    };
    memory
        .remember_configuration(&configuration)
        .expect("deberia guardarse");

    let opened_view = OpenedDocumentView::from(documents::note_opened(
        &memory,
        &opened,
        std::path::PathBuf::from(A_PORTAL_HANDLE),
    ));
    let failure = Failure::from(
        documents::bytes_of(&opened, &opened_view.id)
            .expect_err("el enlace del portal no existe fuera del sandbox"),
    );
    let dropped = DroppedDocumentView::from(
        documents::dropped_document(
            &[
                std::path::PathBuf::from(A_PORTAL_HANDLE),
                std::path::PathBuf::from(ANOTHER_PORTAL_HANDLE),
            ],
            &opened,
        )
        .expect("se ha soltado un fichero"),
    );
    let folder = CheckedFolder::at(home.path()).expect("el temporal esta ahi");
    let refused_rubric =
        crate::documents::adapters::rubric::RubricStore::at(home.path().join("rubric.jpg"))
            .adopt(Path::new(A_PORTAL_HANDLE))
            .expect_err("el enlace del portal no existe fuera del sandbox");

    let mut crossings = vec![
        Serialised::of("OpenedDocumentView", &opened_view),
        Serialised::of("Failure", &failure),
        Serialised::of("DroppedDocumentView", &dropped),
        Serialised::of(
            "DestinationView",
            &DestinationView::from(documents::where_it_lands(&chosen, &document)),
        ),
        Serialised::of(
            "SignedDocumentView",
            &SignedDocumentView::from(documents::told_as(document.reading_path(), &folder, 42)),
        ),
        Serialised::of(
            "ConfigurationView",
            &ConfigurationView::from(configuration::shown(&configuration, home.path())),
        ),
        Serialised::of(
            "RubricChoiceView",
            &crate::documents::adapters::tauri_rubric::RubricChoiceView::refused(&refused_rubric),
        ),
    ];

    let entry: RecentDocument<crate::signing::domain::Spot> =
        serde_json::from_value(serde_json::json!({
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
        crossings.push(Serialised::of(
            "RecentDocumentView",
            &RecentDocumentView::from(row),
        ));
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
            built.contains(output.name) || without.contains(output.name),
            "«{}» ({}) no se construye desde un documento del portal ni esta declarado como \
             tipo sin documento detras: la guarda de rutas no lo mira",
            output.name,
            output.file
        );
    }

    let known: BTreeSet<&str> = outputs.iter().map(|output| output.name).collect();
    for name in built.iter().chain(without.iter()) {
        assert!(
            known.contains(name),
            "«{name}» ya no es un tipo de salida del registro: sobra de la guarda"
        );
    }
}

#[test]
fn every_type_a_command_names_is_in_the_registry() {
    let registered: BTreeSet<&str> = registry().iter().map(|crossing| crossing.name).collect();
    let signatures: Vec<(String, String)> = sources()
        .iter()
        .flat_map(|(_, source)| commands_of(source))
        .map(|(_, name, block)| {
            let signature = block
                .split_once("pub fn ")
                .and_then(|(_, after)| after.split_once('{'))
                .map(|(signature, _)| signature.to_owned())
                .unwrap_or_default();
            (name, signature)
        })
        .collect();
    assert!(signatures.len() >= 14, "el troceado de ordenes no las ve");

    for (name, signature) in &signatures {
        for ty in types_named_by(signature) {
            assert!(
                registered.contains(ty),
                "«{name}» cruza «{ty}» y no esta en el registro: declaralo con `crossing!`, \
                 que un `impl Serialize` a mano no cruza"
            );
        }
    }
}

#[test]
fn every_type_a_crossing_names_is_in_the_registry_and_every_lent_one_is_named() {
    let registry = registry();
    let registered: BTreeSet<&str> = registry.iter().map(|crossing| crossing.name).collect();
    assert_eq!(
        registered.len(),
        registry.len(),
        "un tipo esta registrado dos veces"
    );

    let mut named: BTreeSet<&str> = BTreeSet::new();
    for crossing in registry
        .iter()
        .filter(|crossing| crossing.lent_from.is_none())
    {
        for ty in crossing.referenced_types() {
            if NOT_OURS.contains(&ty) {
                continue;
            }
            assert!(
                registered.contains(ty),
                "«{}» nombra «{ty}» y el contrato no lo explica: declaralo con `crossing!` \
                 o, si es de otro modulo, prestalo con `crossing! {{ lent from … }}`",
                crossing.name
            );
            named.insert(ty);
        }
    }
    for lent in registry
        .iter()
        .filter(|crossing| crossing.lent_from.is_some())
    {
        assert!(
            named.contains(lent.name),
            "«{}» esta prestado y ningun tipo de cruce lo nombra: sobra",
            lent.name
        );
    }
}

#[test]
fn a_type_declared_in_a_test_sibling_does_not_count_as_a_crossing() {
    assert!(all_crossings()
        .iter()
        .any(|crossing| crossing.file.ends_with("/tests.rs")));
    assert!(registry()
        .iter()
        .all(|crossing| !crossing.file.ends_with("/tests.rs")));
}

#[test]
fn the_types_named_by_a_signature_leave_out_the_injected_state_and_the_standard_library() {
    assert_eq!(
        types_named_by(
            "record_recent(id: String, placement: Option<PlacementView>, root: State<'_, DocumentsRoot>, app: tauri::AppHandle) -> Result<RecentDocumentView, Failure>"
        ),
        ["Failure", "PlacementView", "RecentDocumentView"]
    );
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
fn an_adapter_in_a_new_context_is_discovered_without_editing_any_list() {
    let tree = tempfile::tempdir().expect("deberia crearse un directorio temporal");
    for (relative, source) in [
        ("crossing/guards.rs", "pub struct NotThis;\n"),
        ("crossing/tests.rs", "pub struct NotThisEither;\n"),
        (
            "site/adapters/tauri.rs",
            "#[tauri::command]\npub fn two() {}\n",
        ),
        ("site/adapters/tauri/tests.rs", ""),
        ("site/adapters/views.rs", "pub struct AView;\n"),
        (
            "site/adapters/channel/server.rs",
            "pub struct NotThisEither;\n",
        ),
        ("site/domain/errand.rs", "pub struct NotAnAdapter;\n"),
        ("site/application/attend.rs", ""),
        ("memory/recents.rs", ""),
    ] {
        let path = tree.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("tiene carpeta")).expect("carpeta");
        std::fs::write(path, source).expect("fichero");
    }

    let found: Vec<String> = adapter_sources_under(tree.path())
        .into_iter()
        .map(|(relative, _)| relative)
        .collect();

    assert_eq!(found, ["site/adapters/tauri.rs", "site/adapters/views.rs"]);
}

#[test]
fn the_list_of_commands_is_closed_and_this_is_how_long_it_is() {
    let orders: usize = sources()
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
    for (file, command) in [
        ("documents/adapters/tauri.rs", "pub fn open_document("),
        ("documents/adapters/tauri.rs", "pub fn read_document("),
        (
            "documents/adapters/tauri.rs",
            "pub fn open_signed_document(",
        ),
        ("documents/adapters/tauri.rs", "pub fn open_signed_folder("),
        ("desktop/adapters/tauri.rs", "pub fn check_for_new_version("),
    ] {
        let source = production_half(source_of(file));
        let declaration = source
            .find(command)
            .unwrap_or_else(|| panic!("no esta la orden «{command}»"));
        let before = &source[..declaration];
        assert!(
            before.ends_with("#[tauri::command(async)]\n"),
            "«{command}» tiene que ser #[tauri::command(async)]"
        );
    }

    let commands: Vec<_> = sources()
        .iter()
        .flat_map(|(_, source)| commands_of(production_half(source)))
        .collect();
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
    let source = production_half(source_of("site/adapters/tauri.rs"));

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
    let takers: usize = sources()
        .iter()
        .map(|(_, source)| production_half(source).matches("pin: String").count())
        .sum();

    assert_eq!(takers, 1, "el PIN entra por una sola orden");
}
