use super::*;
use crate::fixtures::{a_completed_cycle, a_memory};
use crate::signing::adapters::store::Loaded;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::domain::PageSet;
use std::fs;
use std::path::PathBuf;

fn a_pdf(directory: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, bytes).expect("deberia escribirse");
    path
}

fn an_opened_pdf(directory: &Path, name: &str, opened: &OpenedDocuments) -> (PathBuf, String) {
    let path = a_pdf(directory, name, b"%PDF-1.7 de prueba");
    let id = opened.mint(Document::opened(path.clone()));
    (path, id)
}

fn a_placement(page: u32) -> VisibleBox {
    placed_on(PageSet::only_page(page))
}

fn placed_on(pages: PageSet) -> VisibleBox {
    VisibleBox {
        rect: [72.0, 500.0, 272.0, 600.0],
        pages,
    }
}

#[test]
fn the_tray_survives_being_read_again_with_its_names_badges_and_order() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);

    record(&memory, &opened, &first, None).expect("deberia anotarse");
    record(&memory, &opened, &second, None).expect("deberia anotarse");

    let next_session = OpenedDocuments::new();
    let rows = listed_rows(&memory, &next_session);

    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["nomina.pdf", "contrato.pdf"]);
    assert!(rows.iter().all(|row| row.badge == Badge::Unsigned));
    assert!(rows.iter().all(|row| row.available));
}

#[test]
fn a_path_that_no_longer_answers_is_unavailable_and_revives_when_it_comes_back() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, None).expect("deberia anotarse");

    fs::remove_file(&path).expect("deberia borrarse");
    let gone = listed_rows(&memory, &opened);
    fs::write(&path, b"%PDF-1.7 de vuelta").expect("deberia volver");
    let back = listed_rows(&memory, &opened);

    assert_eq!(gone.len(), 1, "nadie la purga por su cuenta");
    assert!(!gone[0].available);
    assert!(back[0].available, "la fila revive cuando la ruta reaparece");
}

#[test]
fn availability_is_never_written_to_the_disk() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);

    record(&memory, &opened, &id, None).expect("deberia anotarse");

    let written = fs::read_to_string(memory.state_file().path()).expect("deberia leerse");
    assert!(
        !written.contains("available"),
        "«available» es un hecho del disco de ahora mismo y se recalcula al listar: {written}"
    );
}

#[test]
fn only_forget_takes_a_row_out() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);
    record(&memory, &opened, &first, None).expect("deberia anotarse");
    record(&memory, &opened, &second, None).expect("deberia anotarse");

    forget(&memory, &opened, &first).expect("deberia olvidarse");

    let names: Vec<String> = listed_rows(&memory, &opened)
        .into_iter()
        .map(|row| row.name)
        .collect();
    assert_eq!(names, vec!["nomina.pdf".to_owned()]);
}

#[test]
fn a_row_opened_through_a_symlink_is_still_the_row_that_forget_takes_out() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let real = directory.path().join("real");
    fs::create_dir(&real).expect("deberia crearse");
    let linked = directory.path().join("enlace");
    std::os::unix::fs::symlink(&real, &linked).expect("deberia enlazarse");
    a_pdf(&real, "contrato.pdf", b"%PDF-1.7 de prueba");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let id = opened.mint(Document::opened(linked.join("contrato.pdf")));
    record(&memory, &opened, &id, None).expect("deberia anotarse");

    forget(&memory, &opened, &id).expect("deberia olvidarse");

    assert!(listed_rows(&memory, &opened).is_empty());
}

#[test]
fn a_document_that_was_open_before_gets_its_page_and_position_back() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, Some(a_placement(3))).expect("deberia anotarse");

    let again = opened.mint(Document::opened(path));
    let row = record(&memory, &opened, &again, None).expect("deberia anotarse");

    assert_eq!(row.placement, Some(a_placement(3)));
}

#[test]
fn a_brand_new_document_does_not_inherit_the_position_of_another_one() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &first, Some(a_placement(3))).expect("deberia anotarse");

    let (_, second) = an_opened_pdf(directory.path(), "nomina.pdf", &opened);
    let row = record(&memory, &opened, &second, None).expect("deberia anotarse");

    assert_eq!(row.placement, None);
}

#[test]
fn with_the_visible_signature_switch_off_the_box_starts_at_its_default_every_time() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    memory
        .remember_configuration(&Configuration {
            remember_visible_signature: false,
            ..Configuration::default()
        })
        .expect("deberia guardarse");
    let opened = OpenedDocuments::new();
    let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, Some(a_placement(3))).expect("deberia anotarse");

    let again = opened.mint(Document::opened(path));
    let row = record(&memory, &opened, &again, None).expect("deberia anotarse");

    assert_eq!(row.placement, None);
    let state = memory
        .state()
        .map(Loaded::into_value)
        .expect("deberia leerse");
    assert!(state.visible_signature.is_none(), "lo global tampoco");
}

#[test]
fn a_pdf_that_already_carries_signatures_still_enters_as_unsigned() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let path = a_pdf(
        directory.path(),
        "ya-firmado.pdf",
        b"%PDF-1.7\n/ByteRange [0 1000 2000 3000]\n/SubFilter /ETSI.CAdES.detached\n",
    );
    let id = opened.mint(Document::opened(path));

    let row = record(&memory, &opened, &id, None).expect("deberia anotarse");

    assert_eq!(row.badge, Badge::Unsigned);
}

#[test]
fn the_signed_document_is_the_only_row_that_gets_the_signed_badge() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, None).expect("deberia anotarse");
    let landing = a_pdf(
        directory.path(),
        "contrato_firmado.pdf",
        b"%PDF-1.7 firmado",
    );

    note_signed(&memory, &landing, &a_completed_cycle());

    let rows = listed_rows(&memory, &opened);
    let signed: Vec<&str> = rows
        .iter()
        .filter(|row| row.badge == Badge::Signed)
        .map(|row| row.name.as_str())
        .collect();
    assert_eq!(signed, vec!["contrato_firmado.pdf"]);
    assert_eq!(rows.len(), 2, "el original y el firmado son dos ficheros");
}

#[test]
fn reopening_a_document_that_rfirma_signed_does_not_take_its_badge_away() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let landing = a_pdf(
        directory.path(),
        "contrato_firmado.pdf",
        b"%PDF-1.7 firmado",
    );
    note_signed(&memory, &landing, &a_completed_cycle());

    let id = opened.mint(Document::opened(landing));
    let row = record(&memory, &opened, &id, None).expect("deberia anotarse");

    assert_eq!(row.badge, Badge::Signed);
}

#[test]
fn no_row_carries_the_path_the_backend_dedupes_by() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (path, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, None).expect("deberia anotarse");

    let rows = listed_rows(&memory, &opened);

    let told = serde_json::to_string(
        &rows
            .into_iter()
            .map(crate::documents::adapters::views::RecentDocumentView::from)
            .collect::<Vec<_>>(),
    )
    .expect("deberia serializarse");
    assert!(
        !told.contains(&path.to_string_lossy().into_owned()),
        "lo que cruza es el identificador opaco y nada mas: {told}"
    );
    assert!(!told.contains(&directory.path().to_string_lossy().into_owned()));
}

#[test]
fn a_listed_row_can_be_read_because_it_carries_a_usable_identifier() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, None).expect("deberia anotarse");

    let next_session = OpenedDocuments::new();
    let rows = listed_rows(&memory, &next_session);

    let bytes = super::super::documents::bytes_of(&next_session, &rows[0].id)
        .expect("la fila listada tiene que poder abrirse");
    assert!(bytes.starts_with(b"%PDF"));
}

#[test]
fn the_row_of_the_document_in_front_keeps_the_identifier_the_window_already_has() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, id) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &id, None).expect("deberia anotarse");

    let rows = listed_rows(&memory, &opened);

    assert_eq!(rows[0].id, id);
}

#[test]
fn the_size_is_global_and_the_position_is_of_each_document() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (_, first) = an_opened_pdf(directory.path(), "contrato.pdf", &opened);
    record(&memory, &opened, &first, Some(a_placement(1))).expect("deberia anotarse");

    let state = memory
        .state()
        .map(Loaded::into_value)
        .expect("deberia leerse");
    let global = state.visible_signature.expect("el tamano es global");
    assert_eq!(global.size.width, 200.0);
    assert_eq!(global.size.height, 100.0);
    let spot = state.recents.entries()[0]
        .placement()
        .expect("la posicion es de este documento");
    assert_eq!(
        (&spot.pages, spot.lower_left_x),
        (&PageSet::only_page(1), 72.0)
    );
}

#[test]
fn a_document_gets_its_whole_page_set_back_and_not_just_a_page() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(directory.path());
    let opened = OpenedDocuments::new();
    let (path, id) = an_opened_pdf(directory.path(), "expediente.pdf", &opened);
    let placed = placed_on(PageSet::All);
    record(&memory, &opened, &id, Some(placed.clone())).expect("deberia anotarse");

    let again = opened.mint(Document::opened(path));
    let row = record(&memory, &opened, &again, None).expect("deberia anotarse");

    assert_eq!(row.placement, Some(placed));
}

#[test]
fn a_document_that_is_remembered_still_leaves_its_row_when_taken() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let (_, id) = an_opened_pdf(home.path(), "contrato.pdf", &opened);

    let row = take(&memory, &opened, &id, Some(a_placement(3))).expect("deberia ponerse delante");

    assert_eq!(row.name, "contrato.pdf");
    assert_eq!(row.placement, Some(a_placement(3)));
    assert_eq!(listed_rows(&memory, &opened).len(), 1);
}

#[test]
fn a_document_passing_through_leaves_neither_row_nor_placement_when_taken() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let path = home.path().join("de-la-sede.pdf");
    std::fs::write(&path, b"%PDF-1.7\n").expect("deberia escribirse");
    let id = opened.mint(Document::passing_through(path));

    let row = take(&memory, &opened, &id, Some(a_placement(3))).expect("deberia ponerse delante");

    assert_eq!(row.id, id);
    assert_eq!(row.name, "de-la-sede.pdf");
    assert!(row.available);
    assert!(listed_rows(&memory, &opened).is_empty());
    let remembered = memory
        .state()
        .map(Loaded::into_value)
        .ok()
        .and_then(|state| state.visible_signature);
    assert_eq!(
        remembered, None,
        "el tamano del recuadro tampoco se recuerda"
    );
}

#[test]
fn remembrance_decides_the_row_and_belongs_to_the_document_not_to_the_file() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let path = home.path().join("contrato.pdf");
    std::fs::write(&path, b"%PDF-1.7\n").expect("deberia escribirse");
    let passing = opened.mint(Document::passing_through(path.clone()));
    let remembered = opened.mint(Document::opened(path));

    take(&memory, &opened, &passing, None).expect("deberia ponerse delante");
    assert!(listed_rows(&memory, &opened).is_empty());

    take(&memory, &opened, &remembered, None).expect("deberia ponerse delante");
    assert_eq!(listed_rows(&memory, &opened).len(), 1);
}

#[test]
fn the_tray_never_borrows_the_identifier_of_a_document_passing_through() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let (path, remembered) = an_opened_pdf(home.path(), "contrato.pdf", &opened);
    opened.mint(Document::passing_through(path.clone()));
    take(&memory, &opened, &remembered, None).expect("deberia anotarse");

    assert_eq!(identifier_for(&path, &opened), remembered);
}

#[test]
fn an_identifier_of_no_session_puts_nothing_in_hand() {
    let home = tempfile::tempdir().expect("deberia crearse");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    let taken = take(&memory, &opened, "00000000000000000000000000000000", None);

    assert!(taken.is_err());
}
