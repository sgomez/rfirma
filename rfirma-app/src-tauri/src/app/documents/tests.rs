use super::{
    bytes_of, deliver, dropped_document, folder_it_came_from, next_to_the_original, note_opened,
    note_opened_unrecorded, real_path_of, remember_the_folder, remembered_folder, starting_folder,
    told_as, where_it_lands,
};
use crate::app::fixtures::a_memory;
use crate::destination::{CheckedFolder, PortalDocument};
use crate::memory::{Configuration, OpenedDocuments};

fn with_destination(folder: &std::path::Path) -> Configuration {
    Configuration {
        destination: Some(crate::destination::DestinationFolder::at(folder)),
        ..Configuration::default()
    }
}

#[test]
fn what_the_dialog_granted_is_noted_and_read_back_by_its_identifier() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let pdf = home.path().join("contrato.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
    let opened = OpenedDocuments::new();

    let view = note_opened(
        &a_memory(home.path()),
        &Configuration::default(),
        &opened,
        pdf,
    );

    assert_eq!(view.name, "contrato.pdf");
    assert_eq!(view.id.len(), 32);
    assert!(view.modified.is_some(), "el mtime lo lee el backend");
    assert_eq!(
        bytes_of(&opened, &view.id).expect("se leen contra el identificador"),
        b"%PDF-1.4\n"
    );
}

#[test]
fn a_document_that_is_not_open_in_this_session_cannot_be_read() {
    let failure = bytes_of(&OpenedDocuments::new(), "0").expect_err("no esta abierto");

    assert_eq!(failure.situation, "documentUnreadable");
}

#[test]
fn the_identifier_crosses_and_the_reading_path_stays_behind() {
    let opened = OpenedDocuments::new();
    let handle = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

    let id = opened.remember(PortalDocument::opened(handle));

    assert!(
        !id.contains("1e8b83b9"),
        "el identificador no lleva el del portal: {id}"
    );
    assert!(!id.contains("contrato"), "ni el nombre: {id}");
    assert_eq!(
        opened
            .get(&id)
            .map(|document| document.reading_path().to_owned()),
        Some(std::path::PathBuf::from(handle)),
        "y el backend sí sabe por dónde leerlo"
    );
}

#[test]
fn a_dropped_pdf_crosses_as_an_opened_document() {
    let opened = OpenedDocuments::new();
    let pdf = std::env::temp_dir().join("rfirma-commands-soltado.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").expect("se puede escribir en el temporal");

    let view = dropped_document(&[pdf], &opened).expect("algo se ha soltado");

    let document = view.document.expect("y se ha abierto");
    assert_eq!(document.name, "rfirma-commands-soltado.pdf");
    assert_eq!(document.id.len(), 32);
    assert_eq!(view.failure, None);
    assert_eq!(view.discarded, 0);
    assert!(view.also_entering.is_empty());
    assert_eq!(opened.len(), 1);
}

#[test]
fn dropping_something_that_is_not_a_pdf_opens_nothing_and_says_so() {
    let opened = OpenedDocuments::new();
    let other = std::env::temp_dir().join("rfirma-commands-soltado.ods");

    let view = dropped_document(&[other], &opened).expect("algo se ha soltado");

    assert!(view.document.is_none());
    assert_eq!(
        view.failure.map(|failure| failure.situation),
        Some("notAPdf".to_owned())
    );
    assert!(opened.is_empty(), "no se apunta lo que no se abre");
}

#[test]
fn a_dropped_file_the_sandbox_cannot_read_names_its_own_situation() {
    let opened = OpenedDocuments::new();
    let unreachable = std::env::temp_dir().join("rfirma-commands-no-existe/contrato.pdf");

    let view = dropped_document(&[unreachable], &opened).expect("algo se ha soltado");

    let failure = view.failure.expect("se cuenta como un fallo con nombre");
    assert_eq!(failure.situation, "droppedFileUnreadable");
    assert!(!failure.detail.is_empty());
}

#[test]
fn dropping_no_files_at_all_says_nothing() {
    assert_eq!(dropped_document(&[], &OpenedDocuments::new()), None);
}

#[test]
fn every_dropped_pdf_gets_its_own_opened_document_to_enter_recients_with() {
    let opened = OpenedDocuments::new();
    let first = std::env::temp_dir().join("rfirma-commands-primero.pdf");
    let second = std::env::temp_dir().join("rfirma-commands-segundo.pdf");
    std::fs::write(&first, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
    std::fs::write(&second, b"%PDF-1.4\n").expect("se puede escribir en el temporal");

    let view = dropped_document(&[first, second], &opened).expect("algo se ha soltado");

    let document = view.document.expect("el primero se abre");
    assert_eq!(view.also_entering.len(), 1);
    assert_eq!(view.also_entering[0].name, "rfirma-commands-segundo.pdf");
    assert_ne!(document.id, view.also_entering[0].id, "cada uno con su asa");
    assert_eq!(opened.len(), 2);
}

#[test]
fn a_signed_document_is_named_by_its_file_and_its_folder_and_nothing_else() {
    let folder = tempfile::tempdir().expect("deberia haber temporal");
    let checked = CheckedFolder::at(folder.path()).expect("existe");
    let landing = folder.path().join("contrato-firmado.pdf");

    let view = told_as(&landing, &checked, 2_400_000);

    assert_eq!(view.name, "contrato-firmado.pdf");
    assert_eq!(view.size_bytes, 2_400_000);
    assert_eq!(
        view.folder,
        folder.path().file_name().and_then(|n| n.to_str()).unwrap()
    );
    // Ni el nombre ni la carpeta llevan separador de ruta (ADR-0011).
    assert!(!view.name.contains('/'));
    assert!(!view.folder.contains('/'));
}

#[test]
fn the_open_dialog_starts_in_the_destination_folder() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let chosen = documents.path().join("Firmados");
    std::fs::create_dir(&chosen).expect("deberia crearse la carpeta de prueba");

    assert_eq!(
        starting_folder(
            &a_memory(documents.path()),
            &with_destination(&chosen),
            documents.path()
        ),
        Some(chosen)
    );
}

#[test]
fn without_a_chosen_destination_it_starts_in_the_documents_folder() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");

    assert_eq!(
        starting_folder(
            &a_memory(documents.path()),
            &Configuration::default(),
            documents.path()
        ),
        Some(documents.path().to_path_buf())
    );
}

#[test]
fn a_missing_folder_neither_gets_created_nor_stops_the_dialog() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let absent = documents.path().join("Firmados");

    assert_eq!(
        starting_folder(
            &a_memory(documents.path()),
            &with_destination(&absent),
            documents.path()
        ),
        None
    );
    assert!(!absent.exists(), "la carpeta no se puede haber creado");
}

#[test]
fn outside_the_sandbox_the_folder_the_document_came_from_is_the_real_one() {
    let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

    assert_eq!(
        folder_it_came_from(&document),
        Some(std::path::Path::new("/home/quien/Contratos"))
    );
}

#[test]
fn a_document_from_the_portal_leaves_no_folder_to_remember() {
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

    assert_eq!(folder_it_came_from(&document), None);
}

#[test]
fn a_document_with_a_direct_path_offers_the_folder_it_is_in() {
    let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

    let folder = next_to_the_original(&document).expect("hay carpeta original");

    assert_eq!(folder.path(), std::path::Path::new("/home/quien/Contratos"));
    assert_eq!(folder.name(), "Contratos");
}

#[test]
fn a_document_from_the_portal_has_no_original_folder_to_offer() {
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

    assert_eq!(next_to_the_original(&document), None);
}

#[test]
fn outside_the_sandbox_the_real_path_of_the_document_is_told() {
    let document = PortalDocument::opened("/home/quien/Contratos/contrato.pdf");

    assert_eq!(
        real_path_of(&document),
        Some(std::path::Path::new("/home/quien/Contratos/contrato.pdf"))
    );
}

#[test]
fn the_portal_handle_is_never_told_as_the_real_path() {
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf");

    assert_eq!(real_path_of(&document), None);
}

#[test]
fn the_opened_document_crosses_with_the_real_path_only_when_there_is_one() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let pdf = home.path().join("contrato.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    let direct = note_opened(&memory, &Configuration::default(), &opened, pdf.clone());
    let through_the_portal = note_opened(
        &memory,
        &Configuration::default(),
        &opened,
        std::path::PathBuf::from("/run/user/1000/doc/1e8b83b9/contrato.pdf"),
    );

    assert_eq!(direct.path.as_deref(), pdf.to_str());
    assert_eq!(through_the_portal.path, None);
}

#[test]
fn a_document_that_is_not_remembered_does_not_become_the_last_folder_used() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let contracts = home.path().join("Contratos");
    std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
    let pdf = contracts.join("de-la-sede.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    let view = note_opened_unrecorded(&opened, pdf.clone());

    assert_eq!(view.name, "de-la-sede.pdf");
    assert_eq!(bytes_of(&opened, &view.id), Ok(b"%PDF-1.4\n".to_vec()));
    assert_eq!(remembered_folder(&memory), None);
}

#[test]
fn the_same_file_opened_by_the_dialog_does_remember_the_folder() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let contracts = home.path().join("Contratos");
    std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
    let pdf = contracts.join("contrato.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").expect("deberia escribirse el temporal");
    let memory = a_memory(home.path());
    let opened = OpenedDocuments::new();

    note_opened(&memory, &Configuration::default(), &opened, pdf);

    assert_eq!(remembered_folder(&memory), Some(contracts));
}

#[test]
fn the_last_folder_used_wins_over_the_destination_folder() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let contracts = documents.path().join("Contratos");
    std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
    let memory = a_memory(documents.path());
    remember_the_folder(
        &memory,
        &Configuration::default(),
        &PortalDocument::opened(contracts.join("contrato.pdf")),
    );

    assert_eq!(
        starting_folder(&memory, &Configuration::default(), documents.path()),
        Some(contracts)
    );
}

#[test]
fn a_remembered_folder_that_is_gone_falls_back_to_the_destination() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let contracts = documents.path().join("Contratos");
    std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
    let memory = a_memory(documents.path());
    remember_the_folder(
        &memory,
        &Configuration::default(),
        &PortalDocument::opened(contracts.join("contrato.pdf")),
    );
    std::fs::remove_dir(&contracts).expect("deberia borrarse");

    assert_eq!(
        starting_folder(&memory, &Configuration::default(), documents.path()),
        Some(documents.path().to_path_buf())
    );
}

#[test]
fn opening_through_the_portal_never_writes_a_folder_into_the_state() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(documents.path());

    remember_the_folder(
        &memory,
        &Configuration::default(),
        &PortalDocument::opened("/run/user/1000/doc/1e8b83b9/contrato.pdf"),
    );

    assert_eq!(
        memory
            .state()
            .expect("deberia leerse el estado")
            .value()
            .last_open_folder,
        None
    );
    assert_eq!(
        starting_folder(&memory, &Configuration::default(), documents.path()),
        Some(documents.path().to_path_buf())
    );
}

#[test]
fn the_folder_is_not_remembered_with_the_activity_switch_off() {
    let documents = tempfile::tempdir().expect("deberia haber directorio temporal");
    let contracts = documents.path().join("Contratos");
    std::fs::create_dir(&contracts).expect("deberia crearse la carpeta de prueba");
    let memory = a_memory(documents.path());
    let switched_off = Configuration {
        remember_activity: false,
        ..Configuration::default()
    };

    remember_the_folder(
        &memory,
        &switched_off,
        &PortalDocument::opened(contracts.join("contrato.pdf")),
    );

    assert_eq!(
        starting_folder(&memory, &switched_off, documents.path()),
        Some(documents.path().to_path_buf())
    );
}

#[test]
fn the_signed_document_falls_into_the_destination_folder_without_a_dialog() {
    let folder = tempfile::tempdir().expect("deberia haber temporal");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let view = deliver(
        &Configuration::default(),
        folder.path(),
        &document,
        b"%PDF-firmado",
    )
    .expect("cae");

    assert_eq!(view.1.name, "contrato-firmado.pdf");
    assert_eq!(view.1.size_bytes, b"%PDF-firmado".len() as u64);
    assert_eq!(
        std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("esta"),
        b"%PDF-firmado"
    );
}

#[test]
fn a_second_signature_is_numbered_instead_of_overwriting_the_first() {
    let folder = tempfile::tempdir().expect("deberia haber temporal");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    deliver(
        &Configuration::default(),
        folder.path(),
        &document,
        b"la primera",
    )
    .expect("cae");
    let second = deliver(
        &Configuration::default(),
        folder.path(),
        &document,
        b"la segunda",
    )
    .expect("cae tambien");

    assert_ne!(second.1.name, "contrato-firmado.pdf");
    assert_eq!(
        std::fs::read(folder.path().join("contrato-firmado.pdf")).expect("sigue"),
        b"la primera"
    );
}

#[test]
fn a_destination_folder_that_is_not_there_is_told_and_never_created() {
    let missing = tempfile::tempdir()
        .expect("temporal")
        .path()
        .join("no-esta");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let failure =
        deliver(&Configuration::default(), &missing, &document, b"x").expect_err("no esta");

    assert_eq!(failure.situation, "folderMissing");
    assert!(!missing.exists(), "la carpeta se ha creado, y no debía");
}

#[test]
fn the_landing_is_told_by_its_folder_and_its_name_before_signing() {
    let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let view = where_it_lands(
        &with_destination(folder.path()),
        std::path::Path::new("/no/se/usa"),
        &document,
    );

    assert!(view.writable, "la carpeta esta y se puede escribir");
    assert_eq!(view.name.as_deref(), Some("contrato-firmado.pdf"));
    assert_eq!(
        view.folder,
        folder
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .expect("el temporal tiene nombre")
    );
}

#[test]
fn a_namesake_already_there_is_numbered_in_what_the_footer_shows() {
    let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
    std::fs::write(folder.path().join("contrato-firmado.pdf"), b"x")
        .expect("deberia escribirse el homonimo");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let view = where_it_lands(
        &with_destination(folder.path()),
        std::path::Path::new("/no/se/usa"),
        &document,
    );

    assert_eq!(view.name.as_deref(), Some("contrato-firmado-2.pdf"));
}

#[test]
fn a_folder_that_is_not_there_is_told_as_unwritable_and_stays_uncreated() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let missing = home.path().join("Firmados");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let view = where_it_lands(
        &with_destination(&missing),
        std::path::Path::new("/no/se/usa"),
        &document,
    );

    assert!(!view.writable);
    assert_eq!(view.folder, "Firmados", "la carpeta se sigue nombrando");
    assert_eq!(view.name, None, "sin carpeta no hay nombre que prometer");
    assert!(!missing.exists(), "la carpeta se ha creado, y no debía");
}

#[test]
fn telling_the_landing_writes_nothing() {
    let folder = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = PortalDocument::opened("/run/user/1000/doc/1e8b/contrato.pdf");

    let view = where_it_lands(
        &with_destination(folder.path()),
        std::path::Path::new("/no/se/usa"),
        &document,
    );

    assert!(view.name.is_some());
    assert_eq!(
        std::fs::read_dir(folder.path())
            .expect("deberia leerse el temporal")
            .count(),
        0,
        "decidir el destino ha dejado ficheros"
    );
}
