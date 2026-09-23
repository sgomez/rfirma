//! `sign`, `cosign` y `countersign` del cliente publicado en `CAdEStri` contra el servidor trifásico falso de la sede, y sus dos rechazos.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

async fn the_triphase_round_of(script: &str, cop: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;

    let phases: Vec<(&str, &str)> = events
        .iter()
        .filter(|event| event.name() == "triphase")
        .map(|event| (event.field("op"), event.field("cop")))
        .collect();
    assert_eq!(
        phases,
        vec![("pre", cop), ("post", cop)],
        "la prefirma y la postfirma tenian que llegar al serverUrl"
    );
    let conditions: Vec<(&str, &str)> = events
        .iter()
        .filter(|event| event.name() == "condition")
        .map(|event| (event.field("verdict"), event.field("observation")))
        .collect();
    assert_eq!(conditions.len(), 3, "la sede mide tres condiciones");
    assert!(
        conditions
            .iter()
            .all(|(verdict, _)| *verdict == "compliant"),
        "la sede midio: {conditions:?}"
    );
    let verdict = events.last().expect("hay desenlace");
    assert_eq!(verdict.name(), "success", "{}", verdict.field("message"));
}

async fn the_refusal_of(script: &str, code: SafCode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;

    let verdict = events.last().expect("hay desenlace");
    assert_eq!(
        verdict.name(),
        "error",
        "tenia que acabar en el errorCallback"
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(code).on_the_wire()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_signature_goes_through_the_server_url() {
    the_triphase_round_of("signcadestri", "sign").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_cosign_goes_through_the_server_url() {
    the_triphase_round_of("cosigncadestri", "cosign").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_countersign_goes_through_the_server_url() {
    the_triphase_round_of("countersigncadestri", "countersign").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_signature_without_server_url_answers_saf_03() {
    the_refusal_of("signcadestriwithoutserverurl", SafCode::Params).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_server_failure_is_rejected_with_saf_40() {
    the_refusal_of(
        "signcadestriwithafailingserver",
        SafCode::RecoverServerDocument,
    )
    .await;
}
