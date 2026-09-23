//! Selección con `sticky`: preselecciona el certificado, pero nunca contesta sin volver a preguntar (ADR-0010).

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

/// Tres selecciones seguidas del cliente publicado, y las tres preguntan: `sticky` preselecciona, no contesta (ADR-0010).
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
        "la persona vuelve a entregar el mismo certificado"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        2,
        "con sticky la segunda seleccion tambien pregunta: sin ventana no hay certificado"
    );

    let released = client.next_event();
    assert_eq!(
        the_certificate_of(&released, "released"),
        first,
        "tras resetsticky se vuelve a entregar el certificado, ya consentido de nuevo"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        3,
        "tras resetsticky la seleccion vuelve a preguntar"
    );

    let done = client.next_event();
    assert_eq!(done.name(), "done", "el guion tenia que acabar entero");
    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_does_not_spare_the_second_selection_of_the_published_client_from_asking() {
    the_sticky_selections_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_does_not_spare_the_second_selection_over_the_third_protocol_either() {
    the_sticky_selections_of(BenchMode::Third).await;
}
