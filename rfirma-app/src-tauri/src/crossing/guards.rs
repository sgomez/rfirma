//! Guardas de verificación para las órdenes y los tipos que cruzan a la ventana (ADR-0011): las órdenes se leen del fuente, los tipos del registro.

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
const OUTPUTS_WITH_NO_DOCUMENT_BEHIND: [&str; 31] = [
    "StatusView",
    "CertificateView",
    "PlacementView",
    "RubricView",
    "RememberedVisibleSignatureView",
    "VisibleContentView",
    "PhrasePartView",
    "DatumView",
    "SecretView",
    "NewVersionView",
    "SiteErrandView",
    "SiteStageView",
    "LocalBatchItemView",
    "SignatureRoundView",
    "CounterTargetView",
    "SigningKindView",
    "NoChannelView",
    "SiteOutcomeView",
    "RefusalSituationView",
    "NoCertificateView",
    "SignalRowView",
    "StatusActionView",
    "StoreDetailView",
    "StoreCertificatesView",
    "SignalDetailView",
    "SiteSignatureCandidateView",
    "WithdrawalReportView",
    "StoreWithdrawalView",
    "WithdrawalView",
    "SignatureStatusView",
    "ToneView",
];

/// Ruta de prueba simulando un enlace concedido por el portal.
const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

/// Segunda ruta de prueba concedida por el portal.
const ANOTHER_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/segundo.pdf";

/// Ruta de prueba simulando el fichero que concede el diálogo de guardar del portal.
const A_SAVING_GRANT: &str = "/run/user/1000/doc/5a1c02f7/contrato-firmado.pdf";

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

/// Una firma previa de prueba, sin ninguna ruta del portal dentro.
fn a_previous_signature() -> crate::signing::domain::PreviousSignature {
    crate::signing::domain::PreviousSignature {
        name: "LOVELACE BYRON ADA".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        organization_identifier: None,
        issuer: "AC FNMT Usuarios".to_owned(),
        certificate_serial_number: "1234567890".to_owned(),
        signing_time: Some("2024-01-01T10:00:00Z".to_owned()),
        status: crate::signing::domain::SignatureStatus::Valid,
        reason: None,
    }
}

/// Genera todas las salidas producidas a partir de un documento del portal.
#[expect(clippy::too_many_lines)]
fn crossings_from_a_portal_document() -> Vec<Serialised> {
    use crate::crossing::Failure;
    use crate::documents::adapters::files::RealFiles;
    use crate::documents::adapters::views::{
        DestinationView, DroppedDocumentView, OpenedDocumentView, RecentDocumentView,
        SignedDocumentView, SingleDestinationView,
    };
    use crate::documents::application::documents::OpenedDocuments;
    use crate::documents::application::tests::{InMemoryFiles, SavingDialog};
    use crate::documents::application::{documents, recents, single_destination};
    use crate::documents::domain::destination::DestinationFolder;
    use crate::documents::domain::document::Document;
    use crate::documents::domain::recents::Badge;
    use crate::documents::domain::recents::RecentDocument;
    use crate::signing::adapters::state::State;
    use crate::signing::adapters::views::{
        ConfigurationView, PreviousSignatureView, PreviousSignaturesReportView,
    };
    use crate::signing::application::configuration;
    use crate::signing::application::configuration_memory::Configuration;
    use crate::signing::application::tests::a_memory;
    use crate::signing::domain::PreviousSignaturesReport;

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let files = RealFiles;
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
        &files,
        &opened,
        std::path::PathBuf::from(A_PORTAL_HANDLE),
    ));
    let failure = Failure::from(
        documents::bytes_of(&files, &opened, &opened_view.id)
            .expect_err("el enlace del portal no existe fuera del sandbox"),
    );
    let dropped = DroppedDocumentView::from(
        documents::dropped_document(
            &files,
            &[
                std::path::PathBuf::from(A_PORTAL_HANDLE),
                std::path::PathBuf::from(ANOTHER_PORTAL_HANDLE),
            ],
            &opened,
        )
        .expect("se ha soltado un fichero"),
    );
    let folder = documents::checked(&files, &DestinationFolder::at(home.path()))
        .expect("el temporal esta ahi");
    let refused_rubric =
        crate::documents::adapters::rubric::RubricStore::at(home.path().join("rubric.jpg"))
            .adopt(Path::new(A_PORTAL_HANDLE))
            .expect_err("el enlace del portal no existe fuera del sandbox");

    let granted_folder = Path::new(A_SAVING_GRANT)
        .parent()
        .expect("la concesion tiene directorio");
    let singles = single_destination::SingleDestinations::new();
    let disk_behind_the_portal = InMemoryFiles::new().with_folder(granted_folder);
    let chosen_once = single_destination::choose(
        &SavingDialog::answering(A_SAVING_GRANT),
        &disk_behind_the_portal,
        &singles,
        &chosen,
        &document,
    )
    .expect("el dialogo contesta")
    .expect("se ha elegido destino");
    let single = single_destination::chosen(&singles, &chosen_once.id).expect("el asa sigue viva");
    let (_, signed_once) =
        single_destination::deliver(&disk_behind_the_portal, &single, b"%PDF-firmado")
            .expect("cae en la concesion");

    let mut crossings = vec![
        Serialised::of(
            "SingleDestinationView",
            &SingleDestinationView::from(chosen_once),
        ),
        Serialised::of(
            "DestinationView",
            &DestinationView::from(single_destination::where_it_lands(
                &disk_behind_the_portal,
                &single,
            )),
        ),
        Serialised::of("SignedDocumentView", &SignedDocumentView::from(signed_once)),
        Serialised::of("OpenedDocumentView", &opened_view),
        Serialised::of("Failure", &failure),
        Serialised::of("DroppedDocumentView", &dropped),
        Serialised::of(
            "DestinationView",
            &DestinationView::from(documents::where_it_lands(&files, &chosen, &document)),
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
        Serialised::of(
            "PreviousSignatureView",
            &PreviousSignatureView::from(a_previous_signature()),
        ),
        Serialised::of(
            "PreviousSignaturesReportView",
            &PreviousSignaturesReportView::from(PreviousSignaturesReport::new(
                vec![a_previous_signature()],
                false,
            )),
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
    for row in recents::listed_rows(&memory, &files, &opened) {
        crossings.push(Serialised::of(
            "RecentDocumentView",
            &RecentDocumentView::from(row),
        ));
    }

    crossings
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

#[cfg(test)]
mod tests;
