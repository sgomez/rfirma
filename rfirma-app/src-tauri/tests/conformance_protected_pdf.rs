//! `sign` PAdES del cliente publicado sobre un PDF certificado, cifrado o con firmas no registradas, sin nadie a quien preguntar.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

async fn the_refusal_of(script: &str, code: SafCode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;

    let verdict = events.last().expect("hay desenlace");
    assert_eq!(
        verdict.name(),
        "error",
        "'{script}' tenia que acabar en el errorCallback: {}",
        verdict.field("message")
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(code).on_the_wire()
    );
}

async fn the_signature_of(script: &str, measured: usize) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;

    let verdict = events.last().expect("hay desenlace");
    assert_eq!(
        verdict.name(),
        "success",
        "'{script}' tenia que acabar en el successCallback: {}",
        verdict.field("message")
    );
    let conditions: Vec<(&str, &str)> = events
        .iter()
        .filter(|event| event.name() == "condition")
        .map(|event| (event.field("verdict"), event.field("observation")))
        .collect();
    assert_eq!(conditions.len(), measured, "la sede midio: {conditions:?}");
    assert!(
        conditions
            .iter()
            .all(|(verdict, _)| *verdict == "compliant"),
        "la sede midio: {conditions:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_certified_pdf_answers_saf_50_when_headless() {
    the_refusal_of("signpadescertifiedheadless", SafCode::ConfirmationNeeded).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_password_protected_pdf_answers_saf_50_when_headless() {
    the_refusal_of("signpadesprotectedheadless", SafCode::ConfirmationNeeded).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_pdf_with_unregistered_signatures_answers_saf_50_when_headless() {
    the_refusal_of("signpadesunregisteredheadless", SafCode::ConfirmationNeeded).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_wrong_pdf_password_answers_saf_50_when_headless() {
    the_refusal_of(
        "signpadeswrongpasswordheadless",
        SafCode::ConfirmationNeeded,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn allow_signing_certified_pdfs_unlocks_a_certified_pdf() {
    the_signature_of("signpadescertifiedallowed", 1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_password_in_the_request_unlocks_a_protected_pdf() {
    the_signature_of("signpadesprotectedwithitspassword", 0).await;
}
