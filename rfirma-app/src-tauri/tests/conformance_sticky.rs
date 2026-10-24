//! Selección con `sticky`: la segunda contesta con el certificado fijado sin volver a preguntar (ADR-0010).

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// Tres selecciones seguidas del cliente publicado: la fijada no pregunta y la que sigue a `resetsticky` sí (ADR-0010).
async fn the_sticky_selections_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let consents = Arc::new(AtomicUsize::new(0));
    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_the_script(&material, mode, THE_STICKY_SELECTIONS);

    let channel =
        the_errand_channel(&client, &material, &roots, the_errand_of(&roots, &consents)).await;

    let stuck = client.next_event();
    let first = the_certificate_of(&stuck, "stuck");
    assert_eq!(
        consents.load(Ordering::SeqCst),
        1,
        "la primera seleccion siempre pregunta"
    );

    let again = client.next_event();
    assert_eq!(
        the_certificate_of(&again, "stuck-again"),
        first,
        "el fijado vuelve sin que la persona lo entregue"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        1,
        "con sticky la segunda seleccion no pregunta"
    );

    let released = client.next_event();
    assert_eq!(
        the_certificate_of(&released, "released"),
        first,
        "tras resetsticky se vuelve a entregar el certificado, ya consentido de nuevo"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        2,
        "tras resetsticky la seleccion vuelve a preguntar"
    );

    let done = client.next_event();
    assert_eq!(done.name(), "done", "el guion tenia que acabar entero");
    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_spares_the_second_selection_of_the_published_client_from_asking() {
    the_sticky_selections_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_spares_the_second_selection_over_the_third_protocol_too() {
    the_sticky_selections_of(BenchMode::Third).await;
}
