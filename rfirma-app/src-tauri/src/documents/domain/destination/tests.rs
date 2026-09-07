use super::*;

const DOCUMENTS: &str = "/home/quien/Documentos";

fn a_checked_folder() -> CheckedFolder {
    CheckedFolder::confirmed(DOCUMENTS, FolderFact::Folder).expect("deberia comprobarse")
}

fn a_document() -> Document {
    Document::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf")
}

fn first_candidates(document: &Document, how_many: usize) -> Vec<PathBuf> {
    a_checked_folder()
        .landing_candidates(document)
        .take(how_many)
        .collect()
}

#[test]
fn a_folder_that_is_there_can_be_checked_and_shows_its_name() {
    let checked = a_checked_folder();

    assert_eq!(checked.path(), Path::new(DOCUMENTS));
    assert_eq!(checked.name(), "Documentos");
}

#[test]
fn a_folder_that_is_not_there_is_told_by_its_situation() {
    let failure = CheckedFolder::confirmed(DOCUMENTS, FolderFact::Missing)
        .expect_err("no deberia comprobarse");

    assert_eq!(failure.situation(), Situation::FolderMissing);
    assert!(failure.detail().contains("Documentos"));
}

#[test]
fn a_file_where_the_folder_should_be_is_not_a_destination() {
    let failure = CheckedFolder::confirmed(DOCUMENTS, FolderFact::NotAFolder)
        .expect_err("no deberia comprobarse");

    assert_eq!(failure.situation(), Situation::NotAFolder);
}

#[test]
fn a_folder_that_cannot_be_read_drags_the_detail_the_system_gave() {
    let failure =
        CheckedFolder::confirmed(DOCUMENTS, FolderFact::Unreadable("denegado".to_owned()))
            .expect_err("no deberia comprobarse");

    assert_eq!(failure.situation(), Situation::FolderUnreadable);
    assert!(failure.detail().contains("denegado"));
}

#[test]
fn the_signed_document_lands_in_the_destination_folder_with_no_dialogue() {
    assert_eq!(
        first_candidates(&a_document(), 1),
        vec![PathBuf::from("/home/quien/Documentos/contrato-firmado.pdf")]
    );
}

#[test]
fn the_landing_never_falls_next_to_the_original() {
    let document = a_document();

    let landing = first_candidates(&document, 1).remove(0);

    assert_eq!(landing.parent(), Some(Path::new(DOCUMENTS)));
    assert_ne!(landing.parent(), document.reading_path().parent());
}

#[test]
fn a_namesake_is_numbered_instead_of_overwritten() {
    assert_eq!(
        first_candidates(&a_document(), 3),
        vec![
            PathBuf::from("/home/quien/Documentos/contrato-firmado.pdf"),
            PathBuf::from("/home/quien/Documentos/contrato-firmado-2.pdf"),
            PathBuf::from("/home/quien/Documentos/contrato-firmado-3.pdf"),
        ]
    );
}

#[test]
fn cosigning_the_signed_document_does_not_stack_a_second_suffix() {
    let already_signed = Document::opened("/run/user/1000/doc/aa/contrato-firmado.pdf");

    assert_eq!(
        first_candidates(&already_signed, 1),
        vec![PathBuf::from("/home/quien/Documentos/contrato-firmado.pdf")]
    );
}

#[test]
fn the_third_cosignature_keeps_counting_instead_of_stacking() {
    let signed_twice = Document::opened("/run/user/1000/doc/aa/contrato-firmado-2.pdf");

    assert_eq!(
        first_candidates(&signed_twice, 2),
        vec![
            PathBuf::from("/home/quien/Documentos/contrato-firmado.pdf"),
            PathBuf::from("/home/quien/Documentos/contrato-firmado-2.pdf"),
        ]
    );
}

#[test]
fn the_names_run_out_after_the_last_namesake() {
    assert_eq!(
        a_checked_folder().landing_candidates(&a_document()).count(),
        MAX_NAMESAKES as usize
    );
    assert_eq!(
        a_checked_folder().no_free_name(&a_document()).situation(),
        Situation::NoFreeName
    );
}

#[test]
fn the_destination_shows_its_name_and_not_its_path() {
    let folder = DestinationFolder::at("/home/quien/Documentos/Firmados");

    assert_eq!(folder.name(), "Firmados");
}
