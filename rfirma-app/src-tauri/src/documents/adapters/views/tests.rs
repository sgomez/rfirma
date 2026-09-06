use super::{OpenedDocumentView, SignedDocumentView};

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
