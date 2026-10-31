//! Prueba de grada C de la firma y la vista previa de la ventana principal con el certificado de curva elíptica, contra el puente real y el token (ADR-0001).

#[path = "native_cycle/support.rs"]
mod support;

use base64::Engine;
use rfirma_lib::documents::domain::document::Document;
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::signing::adapters::files::RealDocumentBytes;
use rfirma_lib::signing::adapters::isolate::Isolate;
use rfirma_lib::signing::application::{preview, session};
use rfirma_lib::signing::domain::document_signatures::Validity;
use rfirma_lib::signing::domain::SigningChoice;

use support::{a_one_page_pdf, bridge, certificate_labelled, library, ACTIVE_EC, PIN};

use rfirma_lib::signing::adapters::ffi::NativeBridge;

fn the_isolate() -> Isolate {
    Isolate::start_with(|| NativeBridge::open_at(&library()))
}

fn a_pdf_on_disk(directory: &tempfile::TempDir) -> Document {
    let path = directory.path().join("contrato.pdf");
    std::fs::write(&path, a_one_page_pdf()).expect("el PDF deberia escribirse");
    Document::opened(path)
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn a_pades_signed_from_the_main_window_with_the_ec_certificate_verifies() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_pdf_on_disk(&directory);
    let certificate = certificate_labelled(ACTIVE_EC);
    let isolate = the_isolate();
    let session = session::SigningSession::default();

    session::begin(
        &RealDocumentBytes,
        session::DocumentToSign {
            handle: "contrato".to_owned(),
            document,
        },
        &certificate,
        &SigningChoice::for_the_site(false),
        &RealToken,
        &isolate,
        &session,
    )
    .unwrap_or_else(|failure| panic!("el ciclo deberia abrirse: {failure:?}"));
    session::sign_on_token(&RealToken, &session, &ProtectedSecret::from_str(PIN))
        .unwrap_or_else(|failure| panic!("el token deberia firmar con ECDSA: {failure:?}"));
    let signed = session::finish(&isolate, &session)
        .unwrap_or_else(|failure| panic!("la postfirma deberia salir: {failure:?}"));

    let pdf = signed.completed.into_signed_document();
    let report = bridge()
        .previous_signatures(&base64::engine::general_purpose::STANDARD.encode(&pdf))
        .expect("el puente deberia leer las firmas");
    assert_eq!(report.signatures()[0].validity, Validity::Valid);
}

#[test]
#[ignore = "grada C: necesita el token y librfirma_crypto.so (just test-native)"]
fn the_dry_run_with_the_ec_certificate_composes_the_pdf_without_the_pin() {
    let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
    let document = a_pdf_on_disk(&directory);
    let original = a_one_page_pdf();

    let previewed = preview::compose(
        &RealDocumentBytes,
        &document,
        &certificate_labelled(ACTIVE_EC),
        &SigningChoice::for_the_site(false),
        &the_isolate(),
    )
    .unwrap_or_else(|failure| panic!("la vista previa deberia componerse: {failure:?}"));

    assert!(previewed.starts_with(b"%PDF-"));
    assert!(previewed.len() > original.len());
}
