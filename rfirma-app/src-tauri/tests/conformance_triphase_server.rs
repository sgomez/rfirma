//! `sign`, `cosign` y `countersign` del cliente publicado en `CAdEStri` contra el servidor trifásico falso de la sede, y sus dos rechazos.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// El trámite de una firma contra el servidor trifásico: consiente, abre el secreto por la única puerta del PIN y entrega.
fn the_triphase_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSign(consent)) = errand::attend(&desk, url, answering, live)
        else {
            return;
        };
        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just certs install`")
            });

        if errand::consent(&desk, &chosen.id, live).is_err() {
            return;
        }
        tokio::task::block_in_place(|| {
            if signed_with_the_secret(&desk, live, THE_TOKEN_SECRET).is_ok() {
                errand::finish(&desk, live).expect("la firma del servidor deberia entregarse");
            }
        });
    })
}

async fn the_events_of(script: &str) -> Vec<Event> {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let client = PublishedClient::running_the_script(&material, BenchMode::Fourth, script);

    let channel =
        the_errand_channel(&client, &material, &roots, the_triphase_errand_of(&roots)).await;

    let mut events = Vec::new();
    loop {
        let event = client.next_event_or_condition();
        let settled = matches!(event.name(), "success" | "error");
        events.push(event);
        if settled {
            break;
        }
    }
    channel.close();
    events
}

async fn the_triphase_round_of(script: &str, cop: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let events = the_events_of(script).await;

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

    let events = the_events_of(script).await;

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
