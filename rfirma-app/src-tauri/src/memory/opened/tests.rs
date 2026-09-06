use super::*;

const A_PORTAL_HANDLE: &str = "/run/user/1000/doc/1e8b83b9/contrato.pdf";

#[test]
fn an_opened_document_comes_back_by_its_identifier() {
    let opened = OpenedDocuments::new();

    let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

    assert_eq!(
        opened.get(&id),
        Some(PortalDocument::opened(A_PORTAL_HANDLE))
    );
}

#[test]
fn an_identifier_nobody_minted_is_simply_not_there() {
    let opened = OpenedDocuments::new();

    assert_eq!(opened.get("00000000000000000000000000000000"), None);
    assert!(opened.is_empty());
}

#[test]
fn documents_opened_one_after_another_all_stay_open() {
    let opened = OpenedDocuments::new();

    let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
    let second = opened.remember(PortalDocument::opened(
        "/run/user/1000/doc/2f9c94ca/factura.pdf",
    ));

    assert_ne!(first, second);
    assert_eq!(opened.len(), 2);
    assert_eq!(
        opened
            .get(&first)
            .map(|document| document.name().to_owned()),
        Some("contrato.pdf".to_owned())
    );
    assert_eq!(
        opened
            .get(&second)
            .map(|document| document.name().to_owned()),
        Some("factura.pdf".to_owned())
    );
}

#[test]
fn a_grant_says_whether_the_document_it_stands_for_is_remembered() {
    let opened = OpenedDocuments::new();

    let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
    let unrecorded = opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

    assert_eq!(
        opened.remembrance(&remembered),
        Some(Remembrance::Remembered)
    );
    assert_eq!(
        opened.remembrance(&unrecorded),
        Some(Remembrance::Unrecorded)
    );
    assert_eq!(opened.remembrance("00000000000000000000000000000000"), None);
}

#[test]
fn the_tray_never_borrows_the_identifier_of_a_document_that_is_not_remembered() {
    let opened = OpenedDocuments::new();
    let remembered = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
    opened.remember_unrecorded(PortalDocument::opened(A_PORTAL_HANDLE));

    assert_eq!(
        opened.last_id_of(Path::new(A_PORTAL_HANDLE)),
        Some(remembered)
    );
}

#[test]
fn the_identifier_carries_nothing_of_the_path_it_stands_for() {
    let opened = OpenedDocuments::new();

    let id = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

    assert_eq!(id.len(), 32);
    assert!(id.chars().all(|character| character.is_ascii_hexdigit()));
    for leak in ["/", "run", "doc", "1e8b83b9", "contrato", "pdf"] {
        assert!(
            !id.contains(leak),
            "el identificador «{id}» lleva «{leak}» dentro"
        );
    }
}

#[test]
fn the_same_document_opened_twice_is_minted_twice() {
    let opened = OpenedDocuments::new();

    let first = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));
    let second = opened.remember(PortalDocument::opened(A_PORTAL_HANDLE));

    assert_ne!(first, second);
}
