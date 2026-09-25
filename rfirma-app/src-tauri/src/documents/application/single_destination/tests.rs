use std::path::{Path, PathBuf};

use super::{choose, chosen, deliver, where_it_lands, SingleDestinations};
use crate::documents::application::documents::chosen_folder;
use crate::documents::application::tests::{InMemoryFiles, SavingDialog};
use crate::documents::domain::destination::{DestinationFolder, Situation};
use crate::documents::domain::document::Document;
use crate::documents::domain::error::DocumentError;
use crate::documents::ports::DocumentsMemory;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;

const DOCUMENTS: &str = "/home/quien/Documentos";
const CONTRACTS: &str = "/home/quien/Contratos";
const THE_CONTRACT: &str = "/home/quien/Contratos/contrato.pdf";
const A_PORTAL_GRANT: &str = "/run/user/1000/doc/5a1c02f7/acuerdo.pdf";

fn a_disk() -> InMemoryFiles {
    InMemoryFiles::new()
        .with_folder(DOCUMENTS)
        .with_folder(CONTRACTS)
        .with_file(THE_CONTRACT, b"%PDF-1.4\n")
}

fn where_the_memory_lives() -> tempfile::TempDir {
    tempfile::tempdir().expect("deberia haber directorio temporal")
}

#[test]
fn the_destination_chosen_for_one_signature_is_used_and_the_folder_preference_stays() {
    let home = where_the_memory_lives();
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            destination: Some(DestinationFolder::at(DOCUMENTS)),
            ..Configuration::default()
        })
        .expect("deberia guardarse");
    let files = a_disk();
    let singles = SingleDestinations::new();
    let document = Document::opened(THE_CONTRACT);

    let picked = choose(
        &SavingDialog::answering("/home/quien/Contratos/acuerdo.pdf"),
        &files,
        &singles,
        &chosen_folder(&memory, DOCUMENTS),
        &document,
    )
    .expect("el dialogo contesta")
    .expect("se ha elegido destino");
    let single = chosen(&singles, &picked.id).expect("el asa sigue viva");
    let (landing, told) = deliver(&files, &single, b"%PDF-firmado").expect("cae");

    assert_eq!(landing, Path::new("/home/quien/Contratos/acuerdo.pdf"));
    assert_eq!(told.name, "acuerdo.pdf");
    assert_eq!(told.folder, "Contratos");
    assert_eq!(files.written(&landing), Some(b"%PDF-firmado".to_vec()));
    assert_eq!(files.count_in(Path::new(DOCUMENTS)), 0);
    assert_eq!(
        memory.chosen_destination(),
        Some(DestinationFolder::at(DOCUMENTS)),
        "la preferencia de carpeta no se toca"
    );
}

#[test]
fn the_save_dialog_starts_in_the_destination_folder_with_the_name_it_would_get() {
    let files = a_disk().with_file(format!("{DOCUMENTS}/contrato-firmado.pdf"), b"%PDF");
    let dialog = SavingDialog::closed();

    choose(
        &dialog,
        &files,
        &SingleDestinations::new(),
        &DestinationFolder::at(DOCUMENTS),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta");

    let clues = dialog.clues();
    assert_eq!(clues.starting_folder, Some(PathBuf::from(DOCUMENTS)));
    assert_eq!(clues.filename.as_deref(), Some("contrato-firmado-2.pdf"));
    assert_eq!(clues.extensions, vec!["pdf".to_owned()]);
}

#[test]
fn a_destination_folder_that_is_gone_is_not_offered_as_the_place_to_start() {
    let dialog = SavingDialog::closed();

    choose(
        &dialog,
        &a_disk(),
        &SingleDestinations::new(),
        &DestinationFolder::at("/home/quien/Borrada"),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta");

    let clues = dialog.clues();
    assert_eq!(clues.starting_folder, None);
    assert_eq!(clues.filename.as_deref(), Some("contrato-firmado.pdf"));
}

#[test]
fn closing_the_save_dialog_chooses_nothing() {
    let singles = SingleDestinations::new();

    let picked = choose(
        &SavingDialog::closed(),
        &a_disk(),
        &singles,
        &DestinationFolder::at(DOCUMENTS),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta");

    assert_eq!(picked, None);
    assert!(singles.is_empty());
}

#[test]
fn the_chosen_destination_is_told_by_its_folder_and_its_name() {
    let picked = choose(
        &SavingDialog::answering("/home/quien/Contratos/acuerdo.pdf"),
        &a_disk(),
        &SingleDestinations::new(),
        &DestinationFolder::at(DOCUMENTS),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta")
    .expect("se ha elegido destino");

    assert_eq!(picked.destination.folder, "Contratos");
    assert_eq!(picked.destination.name.as_deref(), Some("acuerdo.pdf"));
    assert!(picked.destination.writable);
}

#[test]
fn a_chosen_destination_whose_folder_is_not_there_is_unwritable_and_not_written() {
    let files = a_disk();
    let singles = SingleDestinations::new();
    let picked = choose(
        &SavingDialog::answering("/home/quien/Borrada/acuerdo.pdf"),
        &files,
        &singles,
        &DestinationFolder::at(DOCUMENTS),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta")
    .expect("se ha elegido destino");

    let single = chosen(&singles, &picked.id).expect("el asa sigue viva");
    let told = where_it_lands(&files, &single);

    assert!(!told.writable);
    assert_eq!(told.name.as_deref(), Some("acuerdo.pdf"));
    assert!(deliver(&files, &single, b"%PDF-firmado").is_err());
}

#[test]
fn under_the_portal_the_window_gets_the_name_and_no_folder() {
    let files = a_disk().with_folder("/run/user/1000/doc/5a1c02f7");
    let singles = SingleDestinations::new();

    let picked = choose(
        &SavingDialog::answering(A_PORTAL_GRANT),
        &files,
        &singles,
        &DestinationFolder::at(DOCUMENTS),
        &Document::opened(THE_CONTRACT),
    )
    .expect("el dialogo contesta")
    .expect("se ha elegido destino");
    let single = chosen(&singles, &picked.id).expect("el asa sigue viva");
    let (landing, told) = deliver(&files, &single, b"%PDF-firmado").expect("cae");

    assert_eq!(picked.destination.folder, "");
    assert_eq!(picked.destination.name.as_deref(), Some("acuerdo.pdf"));
    assert_ne!(picked.id, A_PORTAL_GRANT);
    assert_eq!(landing, Path::new(A_PORTAL_GRANT));
    assert_eq!(told.folder, "");
}

#[test]
fn a_destination_that_is_not_in_this_session_is_refused() {
    let refused = chosen(&SingleDestinations::new(), "desconocido").expect_err("no hay asa");

    let DocumentError::Destination(error) = refused else {
        panic!("tiene que ser un fallo del destino: {refused:?}");
    };
    assert_eq!(error.situation(), Situation::FolderMissing);
}
