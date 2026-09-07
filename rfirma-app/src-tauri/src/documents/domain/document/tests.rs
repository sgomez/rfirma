use super::*;
use std::fs;
use std::time::SystemTime;

use crate::documents::domain::recents::{Badge, RecentDocument, ShownBadge};
use crate::signing::domain::Spot;

const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/original.pdf";

#[test]
fn a_document_from_the_portal_yields_its_name_and_its_identifier() {
    let document = Document::opened(A_PORTAL_HANDLE);

    assert_eq!(document.name(), "original.pdf");
    assert_eq!(document.portal_id(), Some("1e8b83b9"));
    assert_eq!(document.origin(), &Origin::Portal("1e8b83b9".to_owned()));
    assert!(document.came_through_the_portal());
    assert!(document.is_remembered());
}

#[test]
fn a_document_passing_through_leaves_no_trace_whatever_its_origin() {
    let from_the_portal = Document::passing_through(A_PORTAL_HANDLE);
    let from_the_host = Document::passing_through("/tmp/rfirma/de-paso.pdf");

    assert!(!from_the_portal.is_remembered());
    assert!(!from_the_host.is_remembered());
    assert!(from_the_portal.came_through_the_portal());
    assert_eq!(from_the_host.origin(), &Origin::Host);
}

#[test]
fn remembrance_belongs_to_the_document_and_not_to_the_path() {
    let remembered = Document::opened(A_PORTAL_HANDLE);
    let passing = Document::passing_through(A_PORTAL_HANDLE);

    assert_eq!(remembered.reading_path(), passing.reading_path());
    assert_ne!(remembered, passing);
}

#[test]
fn a_path_outside_the_portal_has_no_identifier_and_is_still_readable() {
    let document = Document::opened("/home/quien/Documentos/original.pdf");

    assert_eq!(document.name(), "original.pdf");
    assert_eq!(document.portal_id(), None);
    assert_eq!(document.origin(), &Origin::Host);
    assert!(!document.came_through_the_portal());
    assert_eq!(
        document.reading_path(),
        Path::new("/home/quien/Documentos/original.pdf")
    );
}

#[test]
fn a_folder_named_doc_elsewhere_is_not_the_portal() {
    let document = Document::opened("/home/quien/doc/1e8b83b9/original.pdf");

    assert_eq!(document.portal_id(), None);
}

#[test]
fn inside_the_sandbox_the_original_folder_cannot_be_offered() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let marker = directory.path().join(".flatpak-info");
    fs::write(&marker, b"[Application]\n").expect("deberia escribirse");

    assert!(inside_a_sandbox(&marker));
}

#[test]
fn outside_the_sandbox_the_original_folder_can_be_offered() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

    assert!(!inside_a_sandbox(&directory.path().join(".flatpak-info")));
}

#[test]
fn the_question_asked_to_the_environment_is_the_marker_of_the_sandbox() {
    assert_eq!(SANDBOX_MARKER, "/.flatpak-info");
    assert_eq!(
        the_original_folder_can_be_offered(),
        !Path::new("/.flatpak-info").exists()
    );
}

#[test]
fn the_document_that_came_in_never_offers_a_folder_to_write_into() {
    let document = Document::opened(A_PORTAL_HANDLE);

    assert_eq!(document.name(), "original.pdf");
    assert_eq!(document.portal_id(), Some("1e8b83b9"));
    assert_eq!(document.reading_path(), Path::new(A_PORTAL_HANDLE));
}

#[test]
fn a_new_file_at_the_same_path_keeps_the_row_available() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let path = directory.path().join("contrato.pdf");
    fs::write(&path, b"%PDF-1.7 primero").expect("deberia escribirse");
    let entry = RecentDocument::<Spot>::seen(&path, Badge::Unsigned, SystemTime::now())
        .expect("deberia anotarse");

    fs::remove_file(&path).expect("deberia borrarse");
    fs::write(&path, b"%PDF-1.7 otro inodo").expect("deberia escribirse");

    assert!(entry.is_available(), "el permiso va con la ruta");
    assert_eq!(entry.shown_badge(), ShownBadge::Unsigned);
}

#[test]
fn moving_the_file_away_shows_the_unavailable_badge_though_the_inode_lives_on() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let path = directory.path().join("contrato.pdf");
    fs::write(&path, b"%PDF-1.7 de prueba").expect("deberia escribirse");
    let entry = RecentDocument::<Spot>::seen(&path, Badge::Signed, SystemTime::now())
        .expect("deberia anotarse");

    let elsewhere = directory.path().join("archivado.pdf");
    fs::rename(&path, &elsewhere).expect("deberia moverse");

    assert!(elsewhere.exists(), "el inodo sigue vivo");
    assert!(!entry.is_available());
    assert_eq!(entry.shown_badge(), ShownBadge::Unavailable);
}
