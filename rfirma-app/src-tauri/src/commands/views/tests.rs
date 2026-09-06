use super::{
    store_name, CertificateView, OpenedDocumentView, SecretView, SignedDocumentView, StatusView,
};
use crate::pkcs11::{CertificateStatus, StoreClass, StoreSecret};

#[test]
fn the_status_crosses_with_its_payload() {
    let not_yet = StatusView::from(CertificateStatus::NotYetValid { not_before: 42 });
    let unreadable = StatusView::from(CertificateStatus::Unreadable {
        detail: "PEM error".to_owned(),
    });

    assert_eq!(
        serde_json::to_string(&not_yet).expect("serializa"),
        r#"{"kind":"notYetValid","notBefore":42}"#
    );
    assert_eq!(
        serde_json::to_string(&unreadable).expect("serializa"),
        r#"{"kind":"unreadable","detail":"PEM error"}"#
    );
}

#[test]
fn the_secret_crosses_as_one_of_three_kinds_and_never_as_a_string() {
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::NotNeeded)).expect("serializa"),
        r#"{"kind":"notNeeded"}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnScreen {
            attempts_left: None
        }))
        .expect("serializa"),
        r#"{"kind":"typedOnScreen","attemptsLeft":null}"#
    );
    assert_eq!(
        serde_json::to_string(&SecretView::from(StoreSecret::TypedOnTheReaderKeypad))
            .expect("serializa"),
        r#"{"kind":"typedOnTheReaderKeypad"}"#
    );
}

#[test]
fn a_signed_document_is_told_with_two_names_and_its_size() {
    let view = SignedDocumentView {
        name: "contrato_signed.pdf".to_owned(),
        folder: "Documentos".to_owned(),
        size_bytes: 2_400_000,
    };

    assert_eq!(
        serde_json::to_string(&view).expect("serializa"),
        r#"{"name":"contrato_signed.pdf","folder":"Documentos","sizeBytes":2400000}"#
    );
}

#[test]
fn a_certificate_crosses_without_its_der_and_without_its_module() {
    let view = CertificateView {
        id: "0123456789abcdef0123456789abcdef".to_owned(),
        label: "ETIQUETA".to_owned(),
        holder_name: "Ada Lovelace Byron".to_owned(),
        id_number: "IDCES-00000000T".to_owned(),
        issuer: "FNMT-RCM".to_owned(),
        store: store_name(StoreClass::Firefox).to_owned(),
        status: StatusView::Valid {
            not_after: 1_900_000_000,
        },
        remembered: false,
    };
    let json = serde_json::to_string(&view).expect("serializa");

    assert!(json.contains(r#""holderName":"Ada Lovelace Byron""#));
    assert!(!json.contains(r#""der""#), "el DER no sale: {json}");
    assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
}

#[test]
fn the_store_crosses_as_a_class_and_never_as_a_path() {
    let names = [
        store_name(StoreClass::Card),
        store_name(StoreClass::Firefox),
        store_name(StoreClass::Chrome),
        store_name(StoreClass::Nssdb),
        store_name(StoreClass::Installed),
    ];

    assert_eq!(names, ["card", "firefox", "chrome", "nssdb", "installed"]);
    for name in names {
        assert!(!name.contains('/'), "«{name}» parece una ruta");
        assert!(
            name.chars().all(|letter| letter.is_ascii_lowercase()),
            "«{name}» no es una clase en ingles"
        );
    }
}

#[test]
fn an_opened_document_from_the_portal_is_told_without_a_path() {
    let view = OpenedDocumentView {
        id: "0f1e2d3c4b5a69788796a5b4c3d2e1f0".to_owned(),
        name: "contrato.pdf".to_owned(),
        modified: Some(1_700_000_000),
        path: None,
    };

    let json = serde_json::to_string(&view).expect("serializa");

    assert_eq!(
        json,
        r#"{"id":"0f1e2d3c4b5a69788796a5b4c3d2e1f0","name":"contrato.pdf","modified":1700000000,"path":null}"#
    );
    assert!(!json.contains('/'), "no sale ninguna ruta: {json}");
}

#[test]
fn an_opened_document_with_a_direct_path_is_told_with_the_real_one() {
    let view = OpenedDocumentView {
        id: "0f1e2d3c4b5a69788796a5b4c3d2e1f0".to_owned(),
        name: "contrato.pdf".to_owned(),
        modified: Some(1_700_000_000),
        path: Some("/home/quien/Contratos/contrato.pdf".to_owned()),
    };

    let json = serde_json::to_string(&view).expect("serializa");

    assert!(
        json.contains(r#""path":"/home/quien/Contratos/contrato.pdf""#),
        "la ruta real se enseña: {json}"
    );
}
