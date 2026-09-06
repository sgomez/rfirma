use super::*;
use crate::memory::recents::{Badge, Placement, RecentDocument};
use crate::pkcs11::TokenCertificate;
use crate::signing::PageSet;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

fn a_document(directory: &Path) -> RecentDocument {
    let path = directory.join("contrato.pdf");
    fs::write(&path, b"%PDF-1.7 de prueba").expect("deberia escribirse");
    RecentDocument::seen(&path, Badge::Unsigned, SystemTime::now()).expect("deberia anotarse")
}

#[test]
fn a_fresh_state_remembers_nothing() {
    assert!(State::default().is_empty());
}

#[test]
fn forgetting_everything_covers_the_recents_and_the_certificate_alike() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut state = State {
        certificate: Some(CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            "Certificado de pruebas",
            vec![0x01],
        )),
        visible_signature: Some(VisibleSignatureMemory {
            enabled: true,
            size: BoxSize {
                width: 100.0,
                height: 50.0,
            },
            ..VisibleSignatureMemory::default()
        }),
        ..State::default()
    };
    state.recents.record(a_document(directory.path()));

    state.forget_everything();

    assert!(state.is_empty());
    assert!(state.recents.is_empty());
    assert!(state.certificate.is_none());
    assert!(state.visible_signature.is_none());
}

#[test]
fn a_visible_signature_saved_by_an_older_version_survives_the_field_that_is_gone() {
    let saved = r#"{
        "enabled": true,
        "rubric": true,
        "fields": {
            "signer_name": true,
            "id_number": true,
            "signed_at": true,
            "reason": true
        },
        "reason": "Conforme",
        "size": { "width": 200.0, "height": 80.0 }
    }"#;

    let remembered: VisibleSignatureMemory =
        serde_json::from_str(saved).expect("el campo de sobra se ignora");

    assert!(remembered.enabled);
    assert!(remembered.rubric);
    assert_eq!(remembered.reason, "Conforme");
    assert_eq!(remembered.size.width, 200.0);
    assert_eq!(remembered.size.height, 80.0);
    assert_eq!(
        remembered.fields,
        RememberedFields {
            signer_name: true,
            issuer: false,
            signed_at: true,
            reason: true,
        }
    );
}

#[test]
fn forgetting_everything_leaves_the_version_check_alone() {
    let mut state = State {
        last_open_folder: Some(Path::new("/home/quien/Documentos").to_path_buf()),
        version_check: Some(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        }),
        ..State::default()
    };

    state.forget_everything();

    assert!(state.last_open_folder.is_none());
    assert_eq!(
        state.version_check,
        Some(VersionCheck {
            checked_at: 1_756_000_000,
            announced: "0.4.0".to_string(),
        })
    );
}

#[test]
fn what_is_global_and_what_is_of_each_document_are_two_different_places() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut state = State {
        visible_signature: Some(VisibleSignatureMemory {
            enabled: true,
            rubric: true,
            fields: RememberedFields {
                signer_name: true,
                issuer: true,
                signed_at: true,
                reason: false,
            },
            reason: "Conforme".to_owned(),
            size: BoxSize {
                width: 200.0,
                height: 80.0,
            },
        }),
        ..State::default()
    };
    let document = a_document(directory.path());
    let path = document.path().to_path_buf();
    state.recents.record(document);
    state.recents.place(
        &path,
        Some(Placement {
            lower_left_x: 48.0,
            lower_left_y: 179.0,
            pages: PageSet::only_page(3),
        }),
    );

    let written = serde_json::to_value(&state).expect("deberia serializarse");

    let global = &written["visible_signature"];
    assert!(
        global["page"].is_null() && global["lower_left_x"].is_null(),
        "la pagina y la posicion no son globales: son de cada documento"
    );
    assert_eq!(global["size"]["width"], 200.0);
    assert_eq!(global["reason"], "Conforme");
    let row = &written["recents"][0]["placement"];
    assert_eq!(row["pages"], serde_json::json!({ "only": [3] }));
    assert_eq!(row["lower_left_x"], 48.0);
    assert!(
        row["width"].is_null(),
        "el tamano no se repite en cada fila: dos sitios donde divergir"
    );
}

#[test]
fn recording_a_document_again_keeps_where_its_box_had_fallen() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let mut state = State::default();
    let document = a_document(directory.path());
    let path = document.path().to_path_buf();
    state.recents.record(document);
    state.recents.place(
        &path,
        Some(Placement {
            lower_left_x: 10.0,
            lower_left_y: 20.0,
            pages: PageSet::only_page(2),
        }),
    );

    state.recents.record(a_document(directory.path()));

    assert_eq!(
        state.recents.entries()[0]
            .placement()
            .map(|box_| box_.pages.clone()),
        Some(PageSet::only_page(2))
    );
}

#[test]
fn forgetting_everything_also_covers_the_last_folder_opened() {
    let mut state = State {
        last_open_folder: Some(PathBuf::from("/home/quien/Contratos")),
        ..State::default()
    };

    state.forget_everything();

    assert!(state.last_open_folder.is_none());
}

#[test]
fn the_persisted_certificate_is_how_to_find_it_again_and_nothing_else() {
    let state = State {
        certificate: Some(CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            "Certificado de pruebas",
            vec![0x01],
        )),
        ..State::default()
    };

    let written = serde_json::to_value(&state).expect("deberia serializarse");

    let fields: Vec<&str> = written["certificate"]
        .as_object()
        .expect("deberia ser un objeto")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        fields,
        vec!["cka_id", "init_args", "label", "module", "token_label"]
    );
}

#[test]
fn neither_the_holder_nor_the_id_number_reach_the_persisted_file() {
    let holder = "APELLIDO APELLIDO NOMBRE - 12345678Z";
    let mut der = b"\x30\x82 certificado de mentira ".to_vec();
    der.extend_from_slice(holder.as_bytes());
    let certificate = TokenCertificate::new(
        CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            "Certificado de pruebas",
            vec![0x01],
        ),
        der,
    );

    let state = State {
        certificate: Some(certificate.reference().clone()),
        ..State::default()
    };
    let written = serde_json::to_string(&state).expect("deberia serializarse");

    assert!(
        written.contains("libsofthsm2.so") && written.contains("Certificado de pruebas"),
        "se guarda como volver a encontrarlo"
    );
    assert!(
        !written.contains(holder),
        "el titular no puede estar en el fichero"
    );
    assert!(
        !written.contains("12345678Z"),
        "el DNI no puede estar en el fichero"
    );
}
