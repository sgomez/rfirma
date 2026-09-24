//! La firma, y la firma con guardado, del cliente publicado en `CAdEStri`, `PAdEStri`, `XAdEStri` y `FacturaEtri` contra el servidor trifásico falso de la sede, y sus dos rechazos.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

async fn the_triphase_round_of(script: &str, cop: &str, format: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of_a_signing_script(script).await;
    the_triphase_round_in(&events, cop, format);
}

async fn the_triphase_round_saved_by(script: &str, format: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }
    let target = tempfile::tempdir().expect("directorio de guardado");
    let save_path = target.path().join("firmada");

    let events = the_events_of_a_script_saving_to(script, Some(&save_path)).await;
    the_triphase_round_in(&events, "sign", format);

    let returned = STANDARD
        .decode(events.last().expect("hay desenlace").field("result"))
        .expect("la firma llega a la sede en base64");
    let on_disk = std::fs::read(&save_path).expect("la firma tenia que guardarse en disco");
    assert_eq!(
        on_disk, returned,
        "lo guardado en disco es la firma devuelta a la sede"
    );
}

fn the_triphase_round_in(events: &[Event], cop: &str, format: &str) {
    let phases: Vec<(&str, &str, &str)> = events
        .iter()
        .filter(|event| event.name() == "triphase")
        .map(|event| (event.field("op"), event.field("cop"), event.field("format")))
        .collect();
    assert_eq!(
        phases,
        vec![("pre", cop, format), ("post", cop, format)],
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
    the_triphase_round_of("signcadestri", "sign", "CAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_cosign_goes_through_the_server_url() {
    the_triphase_round_of("cosigncadestri", "cosign", "CAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_triphase_countersign_goes_through_the_server_url() {
    the_triphase_round_of("countersigncadestri", "countersign", "CAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_pades_triphase_signature_goes_through_the_server_url() {
    the_triphase_round_of("signpadestri", "sign", "pades").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_xades_triphase_signature_goes_through_the_server_url() {
    the_triphase_round_of("signxadestri", "sign", "XAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn a_facturae_triphase_signature_goes_through_the_server_url() {
    the_triphase_round_of("signfacturaetri", "sign", "FacturaE").await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn signandsave_a_cades_triphase_signature_goes_through_the_server_url_and_is_saved() {
    the_triphase_round_saved_by("signandsavecadestri", "CAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn signandsave_a_pades_triphase_signature_goes_through_the_server_url_and_is_saved() {
    the_triphase_round_saved_by("signandsavepadestri", "pades").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn signandsave_a_xades_triphase_signature_goes_through_the_server_url_and_is_saved() {
    the_triphase_round_saved_by("signandsavexadestri", "XAdES").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn signandsave_a_facturae_triphase_signature_goes_through_the_server_url_and_is_saved() {
    the_triphase_round_saved_by("signandsavefacturaetri", "FacturaE").await;
}
