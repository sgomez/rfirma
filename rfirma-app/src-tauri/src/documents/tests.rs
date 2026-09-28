use std::path::Path;
use std::sync::Arc;

use super::adapters::rubric::RubricStore;
use super::application::documents::OpenedDocuments;
use super::application::single_destination::SingleDestinations;
use super::application::tests::{InMemoryFiles, SavingDialog};
use super::domain::destination::{DestinationFolder, DestinationMode};
use super::domain::document::Document;
use super::ports::DocumentsMemory;
use super::DocumentsRoot;
use crate::signing::adapters::memory::Memory;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;

const DOCUMENTS: &str = "/home/quien/Documentos";
const CONTRACTS: &str = "/home/quien/Contratos";
const THE_CONTRACT: &str = "/home/quien/Contratos/contrato.pdf";
const A_PORTAL_DOCUMENT: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";
const SIGNED: &[u8] = b"%PDF-firmado";

struct Desk {
    root: DocumentsRoot,
    files: Arc<InMemoryFiles>,
    memory: Arc<Memory>,
    _home: tempfile::TempDir,
}

fn a_desk(mode: DestinationMode, files: InMemoryFiles, dialog: SavingDialog) -> Desk {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = Arc::new(a_memory(home.path()));
    memory
        .remember_configuration(&Configuration {
            destination: Some(DestinationFolder::at(DOCUMENTS)),
            destination_mode: mode,
            ..Configuration::default()
        })
        .expect("deberia guardarse");
    let files = Arc::new(files);
    let root = DocumentsRoot {
        documents_folder: DOCUMENTS.into(),
        rubric: RubricStore::at(home.path().join("rubrica.png")),
        opened: OpenedDocuments::new(),
        single_destinations: SingleDestinations::new(),
        memory: memory.clone(),
        files: files.clone(),
        portal: Arc::new(dialog),
    };
    Desk {
        root,
        files,
        memory,
        _home: home,
    }
}

fn a_disk() -> InMemoryFiles {
    InMemoryFiles::new()
        .with_folder(DOCUMENTS)
        .with_folder(CONTRACTS)
        .with_file(THE_CONTRACT, b"%PDF-1.4\n")
}

fn next_to_the_original(files: InMemoryFiles) -> Desk {
    a_desk(
        DestinationMode::NextToTheOriginal,
        files,
        SavingDialog::closed(),
    )
}

#[test]
fn next_to_the_original_a_document_with_a_direct_path_lands_in_its_own_folder() {
    let desk = next_to_the_original(a_disk());

    let (landing, told) = desk
        .root
        .deliver(&Document::opened(THE_CONTRACT), SIGNED, None)
        .expect("cae");

    assert_eq!(
        landing,
        Path::new("/home/quien/Contratos/contrato-firmado.pdf")
    );
    assert_eq!(told.folder, "Contratos");
    assert_eq!(desk.files.written(&landing), Some(SIGNED.to_vec()));
    assert_eq!(desk.files.count_in(Path::new(DOCUMENTS)), 0);
}

#[test]
fn next_to_the_original_a_document_from_the_portal_lands_in_the_destination_folder() {
    let desk = next_to_the_original(a_disk());

    let (landing, told) = desk
        .root
        .deliver(&Document::opened(A_PORTAL_DOCUMENT), SIGNED, None)
        .expect("cae");

    assert_eq!(
        landing,
        Path::new("/home/quien/Documentos/contrato-firmado.pdf")
    );
    assert_eq!(told.folder, "Documentos");
}

#[test]
fn in_the_destination_folder_a_document_with_a_direct_path_lands_there() {
    let desk = a_desk(
        DestinationMode::InTheDestinationFolder,
        a_disk(),
        SavingDialog::closed(),
    );
    let document = Document::opened(THE_CONTRACT);

    let footer = desk
        .root
        .where_it_lands(&document, None)
        .expect("se anuncia");
    let (landing, _) = desk.root.deliver(&document, SIGNED, None).expect("cae");

    assert_eq!(footer.folder, "Documentos");
    assert_eq!(
        landing,
        Path::new("/home/quien/Documentos/contrato-firmado.pdf")
    );
    assert_eq!(
        desk.files.count_in(Path::new(CONTRACTS)),
        1,
        "solo el original"
    );
}

#[test]
fn the_footer_and_the_writing_share_the_folder_next_to_the_original() {
    let desk = next_to_the_original(a_disk());
    let document = Document::opened(THE_CONTRACT);

    let footer = desk
        .root
        .where_it_lands(&document, None)
        .expect("se anuncia");
    let (_, told) = desk.root.deliver(&document, SIGNED, None).expect("cae");

    assert!(footer.writable);
    assert_eq!(footer.folder, told.folder);
    assert_eq!(footer.name.as_deref(), Some(told.name.as_str()));
}

#[test]
fn a_second_signature_next_to_the_original_is_numbered() {
    let desk = next_to_the_original(a_disk());
    let document = Document::opened(THE_CONTRACT);

    desk.root
        .deliver(&document, b"la primera", None)
        .expect("cae");
    let (_, second) = desk
        .root
        .deliver(&document, b"la segunda", None)
        .expect("cae tambien");

    assert_eq!(second.name, "contrato-firmado-2.pdf");
    assert_eq!(
        desk.files
            .written(Path::new("/home/quien/Contratos/contrato-firmado.pdf")),
        Some(b"la primera".to_vec())
    );
}

#[test]
fn an_original_folder_that_cannot_be_written_is_told_in_the_footer_and_never_created() {
    let files = InMemoryFiles::new()
        .with_folder(DOCUMENTS)
        .with_file(THE_CONTRACT, b"%PDF-1.4\n");
    let desk = next_to_the_original(files);
    let document = Document::opened(THE_CONTRACT);

    let footer = desk
        .root
        .where_it_lands(&document, None)
        .expect("se anuncia");
    let refused = desk.root.deliver(&document, SIGNED, None);

    assert!(!footer.writable);
    assert_eq!(footer.folder, "Contratos");
    assert!(refused.is_err(), "no cae en otra parte a escondidas");
    assert_eq!(
        desk.files.count_in(Path::new(CONTRACTS)),
        1,
        "solo el original"
    );
    assert_eq!(desk.files.count_in(Path::new(DOCUMENTS)), 0);
}

#[test]
fn telling_the_landing_next_to_the_original_writes_nothing() {
    let desk = next_to_the_original(a_disk());

    let footer = desk
        .root
        .where_it_lands(&Document::opened(THE_CONTRACT), None)
        .expect("se anuncia");

    assert_eq!(footer.name.as_deref(), Some("contrato-firmado.pdf"));
    assert_eq!(
        desk.files.count_in(Path::new(CONTRACTS)),
        1,
        "solo el original"
    );
}

#[test]
fn the_destination_of_a_single_signature_wins_over_the_mode_and_leaves_it_alone() {
    let desk = a_desk(
        DestinationMode::NextToTheOriginal,
        a_disk(),
        SavingDialog::answering("/home/quien/Documentos/acuerdo.pdf"),
    );
    let document = Document::opened(THE_CONTRACT);
    let picked = desk
        .root
        .choose_single_destination(&document)
        .expect("el dialogo contesta")
        .expect("se ha elegido destino");

    let (landing, _) = desk
        .root
        .deliver(&document, SIGNED, Some(&picked.id))
        .expect("cae");

    assert_eq!(landing, Path::new("/home/quien/Documentos/acuerdo.pdf"));
    assert_eq!(
        desk.files.count_in(Path::new(CONTRACTS)),
        1,
        "solo el original"
    );
    assert_eq!(
        desk.memory.destination_mode(),
        DestinationMode::NextToTheOriginal
    );
}
