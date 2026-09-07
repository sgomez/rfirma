use super::{
    bytes_of, chosen_folder, deliver, dropped_document, folder_it_came_from, next_to_the_original,
    note_opened, note_opened_unrecorded, real_path_of, remember_the_folder, remembered_folder,
    starting_folder, told_as, where_it_lands, OpenedDocuments,
};
use crate::crossing::Failure;
use crate::documents::application::tests::InMemoryFiles;
use crate::documents::domain::destination::{CheckedFolder, DestinationFolder, FolderFact};
use crate::documents::domain::document::Document;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;

const PDF: &[u8] = b"%PDF-1.4\n";
const CONTRACTS: &str = "/home/quien/Contratos";
const DOCUMENTS: &str = "/home/quien/Documentos";
const A_PORTAL_DOCUMENT: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

fn with_destination(folder: &str) -> DestinationFolder {
    DestinationFolder::at(folder)
}

fn a_disk_with_the_contract() -> InMemoryFiles {
    InMemoryFiles::new()
        .with_folder(CONTRACTS)
        .with_file(format!("{CONTRACTS}/contrato.pdf"), PDF)
}

fn where_the_memory_lives() -> tempfile::TempDir {
    tempfile::tempdir().expect("deberia haber directorio temporal")
}

#[test]
fn what_the_dialog_granted_is_noted_and_read_back_by_its_identifier() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract();
    let opened = OpenedDocuments::new();

    let view = note_opened(
        &a_memory(home.path()),
        &files,
        &opened,
        format!("{CONTRACTS}/contrato.pdf").into(),
    );

    assert_eq!(view.name, "contrato.pdf");
    assert_eq!(view.id.len(), 32);
    assert!(view.modified.is_some(), "el mtime lo lee el backend");
    assert_eq!(
        bytes_of(&files, &opened, &view.id).expect("se leen contra el identificador"),
        PDF
    );
}

#[test]
fn a_document_that_is_not_open_in_this_session_cannot_be_read() {
    let failure =
        bytes_of(&InMemoryFiles::new(), &OpenedDocuments::new(), "0").expect_err("no esta abierto");

    assert_eq!(Failure::from(failure).situation, "documentUnreadable");
}

#[test]
fn the_identifier_crosses_and_the_reading_path_stays_behind() {
    let opened = OpenedDocuments::new();

    let id = opened.mint(Document::opened(A_PORTAL_DOCUMENT));

    assert!(
        !id.contains("1e8b83b9"),
        "el identificador no lleva el del portal: {id}"
    );
    assert!(!id.contains("contrato"), "ni el nombre: {id}");
    assert_eq!(
        opened
            .get(&id)
            .map(|document| document.reading_path().to_owned()),
        Some(std::path::PathBuf::from(A_PORTAL_DOCUMENT)),
        "y el backend sí sabe por dónde leerlo"
    );
}

#[test]
fn a_dropped_pdf_crosses_as_an_opened_document() {
    let opened = OpenedDocuments::new();
    let files = a_disk_with_the_contract();

    let view = dropped_document(
        &files,
        &[format!("{CONTRACTS}/contrato.pdf").into()],
        &opened,
    )
    .expect("algo se ha soltado");

    let document = view.document.expect("y se ha abierto");
    assert_eq!(document.name, "contrato.pdf");
    assert_eq!(document.id.len(), 32);
    assert_eq!(view.refused, None);
    assert_eq!(view.discarded, 0);
    assert!(view.also_entering.is_empty());
    assert_eq!(opened.len(), 1);
}

#[test]
fn dropping_something_that_is_not_a_pdf_opens_nothing_and_says_so() {
    let opened = OpenedDocuments::new();
    let files = InMemoryFiles::new()
        .with_folder(CONTRACTS)
        .with_file(format!("{CONTRACTS}/hoja.ods"), b"x");

    let view = dropped_document(&files, &[format!("{CONTRACTS}/hoja.ods").into()], &opened)
        .expect("algo se ha soltado");

    assert!(view.document.is_none());
    assert_eq!(
        view.refused.map(|refused| Failure::from(refused).situation),
        Some("notAPdf".to_owned())
    );
    assert!(opened.is_empty(), "no se apunta lo que no se abre");
}

#[test]
fn a_dropped_file_the_sandbox_cannot_read_names_its_own_situation() {
    let opened = OpenedDocuments::new();
    let files = a_disk_with_the_contract().with_unreadable(format!("{CONTRACTS}/contrato.pdf"));

    let view = dropped_document(
        &files,
        &[format!("{CONTRACTS}/contrato.pdf").into()],
        &opened,
    )
    .expect("algo se ha soltado");

    let failure = Failure::from(view.refused.expect("se cuenta como un fallo con nombre"));
    assert_eq!(failure.situation, "droppedFileUnreadable");
    assert!(!failure.detail.is_empty());
}

#[test]
fn dropping_no_files_at_all_says_nothing() {
    assert_eq!(
        dropped_document(&InMemoryFiles::new(), &[], &OpenedDocuments::new()),
        None
    );
}

#[test]
fn every_dropped_pdf_gets_its_own_opened_document_to_enter_recients_with() {
    let opened = OpenedDocuments::new();
    let files = InMemoryFiles::new()
        .with_folder(CONTRACTS)
        .with_file(format!("{CONTRACTS}/primero.pdf"), PDF)
        .with_file(format!("{CONTRACTS}/segundo.pdf"), PDF);

    let view = dropped_document(
        &files,
        &[
            format!("{CONTRACTS}/primero.pdf").into(),
            format!("{CONTRACTS}/segundo.pdf").into(),
        ],
        &opened,
    )
    .expect("algo se ha soltado");

    let document = view.document.expect("el primero se abre");
    assert_eq!(view.also_entering.len(), 1);
    assert_eq!(view.also_entering[0].name, "segundo.pdf");
    assert_ne!(document.id, view.also_entering[0].id, "cada uno con su asa");
    assert_eq!(opened.len(), 2);
}

#[test]
fn a_dropped_folder_lets_every_pdf_inside_it_enter() {
    let opened = OpenedDocuments::new();
    let files = InMemoryFiles::new()
        .with_folder(CONTRACTS)
        .with_file(format!("{CONTRACTS}/primero.pdf"), PDF)
        .with_file(format!("{CONTRACTS}/hoja.ods"), b"x");

    let view = dropped_document(&files, &[CONTRACTS.into()], &opened).expect("algo se ha soltado");

    assert_eq!(
        view.document.map(|document| document.name),
        Some("primero.pdf".to_owned())
    );
    assert_eq!(view.discarded, 1, "la hoja de cálculo no entra");
}

#[test]
fn a_signed_document_is_named_by_its_file_and_its_folder_and_nothing_else() {
    let checked =
        CheckedFolder::confirmed(DOCUMENTS, FolderFact::Folder).expect("la carpeta esta ahi");

    let view = told_as(
        std::path::Path::new("/home/quien/Documentos/contrato-firmado.pdf"),
        &checked,
        2_400_000,
    );

    assert_eq!(view.name, "contrato-firmado.pdf");
    assert_eq!(view.size_bytes, 2_400_000);
    assert_eq!(view.folder, "Documentos");
    // Ni el nombre ni la carpeta llevan separador de ruta (ADR-0011).
    assert!(!view.name.contains('/'));
    assert!(!view.folder.contains('/'));
}

#[test]
fn the_open_dialog_starts_in_the_destination_folder() {
    let home = where_the_memory_lives();
    let signed = format!("{DOCUMENTS}/Firmados");
    let files = InMemoryFiles::new().with_folder(&signed);

    assert_eq!(
        starting_folder(&a_memory(home.path()), &files, &with_destination(&signed)),
        Some(signed.into())
    );
}

#[test]
fn a_missing_folder_neither_gets_created_nor_stops_the_dialog() {
    let home = where_the_memory_lives();
    let files = InMemoryFiles::new();
    let absent = format!("{DOCUMENTS}/Firmados");

    assert_eq!(
        starting_folder(&a_memory(home.path()), &files, &with_destination(&absent)),
        None
    );
    assert_eq!(files.count_in(std::path::Path::new(&absent)), 0);
}

#[test]
fn outside_the_sandbox_the_folder_the_document_came_from_is_the_real_one() {
    let document = Document::opened(format!("{CONTRACTS}/contrato.pdf"));

    assert_eq!(
        folder_it_came_from(&document),
        Some(std::path::Path::new(CONTRACTS))
    );
}

#[test]
fn a_document_from_the_portal_leaves_no_folder_to_remember() {
    let document = Document::opened(A_PORTAL_DOCUMENT);

    assert_eq!(folder_it_came_from(&document), None);
}

#[test]
fn a_document_with_a_direct_path_offers_the_folder_it_is_in() {
    let document = Document::opened(format!("{CONTRACTS}/contrato.pdf"));

    let folder = next_to_the_original(&document).expect("hay carpeta original");

    assert_eq!(folder.path(), std::path::Path::new(CONTRACTS));
    assert_eq!(folder.name(), "Contratos");
}

#[test]
fn a_document_from_the_portal_has_no_original_folder_to_offer() {
    let document = Document::opened(A_PORTAL_DOCUMENT);

    assert_eq!(next_to_the_original(&document), None);
}

#[test]
fn outside_the_sandbox_the_real_path_of_the_document_is_told() {
    let document = Document::opened(format!("{CONTRACTS}/contrato.pdf"));

    assert_eq!(
        real_path_of(&document),
        Some(std::path::Path::new("/home/quien/Contratos/contrato.pdf"))
    );
}

#[test]
fn the_portal_handle_is_never_told_as_the_real_path() {
    let document = Document::opened(A_PORTAL_DOCUMENT);

    assert_eq!(real_path_of(&document), None);
}

#[test]
fn the_opened_document_crosses_with_the_real_path_only_when_there_is_one() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract();
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();
    let pdf = format!("{CONTRACTS}/contrato.pdf");

    let direct = note_opened(&memory, &files, &opened, pdf.clone().into());
    let through_the_portal = note_opened(&memory, &files, &opened, A_PORTAL_DOCUMENT.into());

    assert_eq!(direct.path.as_deref(), Some(pdf.as_str()));
    assert_eq!(through_the_portal.path, None);
}

#[test]
fn a_document_that_is_not_remembered_does_not_become_the_last_folder_used() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract();
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    let view = note_opened_unrecorded(&files, &opened, format!("{CONTRACTS}/contrato.pdf").into());

    assert_eq!(view.name, "contrato.pdf");
    assert_eq!(bytes_of(&files, &opened, &view.id), Ok(PDF.to_vec()));
    assert_eq!(remembered_folder(&memory, &files), None);
}

#[test]
fn the_same_file_opened_by_the_dialog_does_remember_the_folder() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract();
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    note_opened(
        &memory,
        &files,
        &opened,
        format!("{CONTRACTS}/contrato.pdf").into(),
    );

    assert_eq!(remembered_folder(&memory, &files), Some(CONTRACTS.into()));
}

#[test]
fn the_last_folder_used_wins_over_the_destination_folder() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract().with_folder(DOCUMENTS);
    let memory = a_memory(home.path());
    remember_the_folder(
        &memory,
        &Document::opened(format!("{CONTRACTS}/contrato.pdf")),
    );

    assert_eq!(
        starting_folder(&memory, &files, &with_destination(DOCUMENTS)),
        Some(CONTRACTS.into())
    );
}

#[test]
fn a_remembered_folder_that_is_gone_falls_back_to_the_destination() {
    let home = where_the_memory_lives();
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let memory = a_memory(home.path());
    remember_the_folder(
        &memory,
        &Document::opened(format!("{CONTRACTS}/contrato.pdf")),
    );

    assert_eq!(
        starting_folder(&memory, &files, &with_destination(DOCUMENTS)),
        Some(DOCUMENTS.into())
    );
}

#[test]
fn opening_through_the_portal_never_writes_a_folder_into_the_state() {
    let home = where_the_memory_lives();
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let memory = a_memory(home.path());

    remember_the_folder(&memory, &Document::opened(A_PORTAL_DOCUMENT));

    assert_eq!(
        memory
            .state()
            .expect("deberia leerse el estado")
            .value()
            .last_open_folder,
        None
    );
    assert_eq!(
        starting_folder(&memory, &files, &with_destination(DOCUMENTS)),
        Some(DOCUMENTS.into())
    );
}

#[test]
fn the_folder_is_not_remembered_with_the_activity_switch_off() {
    let home = where_the_memory_lives();
    let files = a_disk_with_the_contract().with_folder(DOCUMENTS);
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            remember_activity: false,
            ..Configuration::default()
        })
        .expect("deberia guardarse");

    remember_the_folder(
        &memory,
        &Document::opened(format!("{CONTRACTS}/contrato.pdf")),
    );

    assert_eq!(
        starting_folder(&memory, &files, &with_destination(DOCUMENTS)),
        Some(DOCUMENTS.into())
    );
}

#[test]
fn the_signed_document_falls_into_the_destination_folder_without_a_dialog() {
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let view = deliver(
        &files,
        &with_destination(DOCUMENTS),
        &document,
        b"%PDF-firmado",
    )
    .expect("cae");

    assert_eq!(view.1.name, "contrato-firmado.pdf");
    assert_eq!(view.1.size_bytes, b"%PDF-firmado".len() as u64);
    assert_eq!(
        files.written(std::path::Path::new(
            "/home/quien/Documentos/contrato-firmado.pdf"
        )),
        Some(b"%PDF-firmado".to_vec())
    );
}

#[test]
fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let document = Document::opened(A_PORTAL_DOCUMENT);

    deliver(
        &files,
        &with_destination(DOCUMENTS),
        &document,
        b"la primera",
    )
    .expect("cae");
    let second = deliver(
        &files,
        &with_destination(DOCUMENTS),
        &document,
        b"la segunda",
    )
    .expect("cae tambien");

    assert_ne!(second.1.name, "contrato-firmado.pdf");
    assert_eq!(
        files.written(std::path::Path::new(
            "/home/quien/Documentos/contrato-firmado.pdf"
        )),
        Some(b"la primera".to_vec())
    );
}

#[test]
fn a_destination_folder_that_is_not_there_is_told_and_never_created() {
    let files = InMemoryFiles::new();
    let missing = format!("{DOCUMENTS}/no-esta");
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let failure =
        deliver(&files, &with_destination(&missing), &document, b"x").expect_err("no esta");

    assert_eq!(Failure::from(failure).situation, "folderMissing");
    assert_eq!(
        files.count_in(std::path::Path::new(&missing)),
        0,
        "la carpeta se ha creado, y no debía"
    );
}

#[test]
fn the_landing_is_told_by_its_folder_and_its_name_before_signing() {
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let view = where_it_lands(&files, &with_destination(DOCUMENTS), &document);

    assert!(view.writable, "la carpeta esta y se puede escribir");
    assert_eq!(view.name.as_deref(), Some("contrato-firmado.pdf"));
    assert_eq!(view.folder, "Documentos");
}

#[test]
fn a_namesake_already_there_is_numbered_in_what_the_footer_shows() {
    let files = InMemoryFiles::new()
        .with_folder(DOCUMENTS)
        .with_file(format!("{DOCUMENTS}/contrato-firmado.pdf"), b"x");
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let view = where_it_lands(&files, &with_destination(DOCUMENTS), &document);

    assert_eq!(view.name.as_deref(), Some("contrato-firmado-2.pdf"));
}

#[test]
fn a_folder_that_is_not_there_is_told_as_unwritable_and_stays_uncreated() {
    let files = InMemoryFiles::new();
    let missing = format!("{DOCUMENTS}/Firmados");
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let view = where_it_lands(&files, &with_destination(&missing), &document);

    assert!(!view.writable);
    assert_eq!(view.folder, "Firmados", "la carpeta se sigue nombrando");
    assert_eq!(view.name, None, "sin carpeta no hay nombre que prometer");
    assert_eq!(files.count_in(std::path::Path::new(&missing)), 0);
}

#[test]
fn telling_the_landing_writes_nothing() {
    let files = InMemoryFiles::new().with_folder(DOCUMENTS);
    let document = Document::opened(A_PORTAL_DOCUMENT);

    let view = where_it_lands(&files, &with_destination(DOCUMENTS), &document);

    assert!(view.name.is_some());
    assert_eq!(
        files.count_in(std::path::Path::new(DOCUMENTS)),
        0,
        "decidir el destino ha dejado ficheros"
    );
}

#[test]
fn the_remembered_folder_is_reused_and_nobody_is_asked_again() {
    let home = where_the_memory_lives();
    let memory = a_memory(home.path());
    memory
        .remember_configuration(&Configuration {
            destination: Some(DestinationFolder::at(format!("{DOCUMENTS}/Firmados"))),
            ..Configuration::default()
        })
        .expect("deberia guardarse");

    let folder = chosen_folder(&memory, DOCUMENTS);

    assert_eq!(
        folder.path(),
        std::path::Path::new("/home/quien/Documentos/Firmados"),
        "elegida una vez, se reutiliza"
    );
    assert_eq!(folder.name(), "Firmados");
}

#[test]
fn without_a_remembered_folder_the_destination_is_the_documents_folder() {
    let home = where_the_memory_lives();

    let folder = chosen_folder(&a_memory(home.path()), DOCUMENTS);

    assert_eq!(folder.path(), std::path::Path::new(DOCUMENTS));
}
