use super::*;
use crate::fixtures::a_memory;
use crate::signing::domain::PageSet;

fn a_pdf(directory: &Path, name: &str) -> std::path::PathBuf {
    let path = directory.join(name);
    std::fs::write(&path, b"%PDF-1.7\n").expect("deberia escribirse");
    path
}

fn a_placement() -> PlacementView {
    PlacementView {
        rect: [10.0, 20.0, 210.0, 70.0],
        pages: PageSet::only_page(3),
    }
}

#[test]
fn a_document_that_is_remembered_still_leaves_its_row() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let configuration = Configuration::default();
    let opened = OpenedDocuments::new();
    let path = a_pdf(home.path(), "contrato.pdf");
    let id = opened.remember(PortalDocument::opened(path));

    let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
        .expect("deberia ponerse delante");

    assert_eq!(row.name, "contrato.pdf");
    assert_eq!(row.placement, Some(a_placement()));
    assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
}

#[test]
fn a_document_that_is_not_remembered_leaves_neither_row_nor_placement() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let configuration = Configuration::default();
    let opened = OpenedDocuments::new();
    let path = a_pdf(home.path(), "de-la-sede.pdf");
    let id = opened.remember_unrecorded(PortalDocument::opened(path));

    let row = take(&memory, &configuration, &opened, &id, Some(a_placement()))
        .expect("deberia ponerse delante");

    assert_eq!(row.id, id);
    assert_eq!(row.name, "de-la-sede.pdf");
    assert!(recents::listed_rows(&memory, &opened).is_empty());
    let remembered = memory
        .state()
        .map(crate::signing::adapters::store::Loaded::into_value)
        .ok()
        .and_then(|state| state.visible_signature);
    assert_eq!(
        remembered, None,
        "el tamano del recuadro tampoco se recuerda"
    );
}

#[test]
fn remembrance_belongs_to_the_grant_and_not_to_the_file() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let configuration = Configuration::default();
    let opened = OpenedDocuments::new();
    let path = a_pdf(home.path(), "contrato.pdf");
    let unrecorded = opened.remember_unrecorded(PortalDocument::opened(path.clone()));
    let remembered = opened.remember(PortalDocument::opened(path));

    take(&memory, &configuration, &opened, &unrecorded, None).expect("deberia ponerse delante");
    assert!(recents::listed_rows(&memory, &opened).is_empty());

    take(&memory, &configuration, &opened, &remembered, None).expect("deberia ponerse delante");
    assert_eq!(recents::listed_rows(&memory, &opened).len(), 1);
}

#[test]
fn an_identifier_of_no_session_puts_nothing_in_hand() {
    let opened = OpenedDocuments::new();

    let taken = DocumentInHand::taken(&opened, "00000000000000000000000000000000");

    assert!(taken.is_err());
}
