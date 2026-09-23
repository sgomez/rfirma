//! Operaciones sobre el canal local de punta a punta: quién las contesta, en qué orden y a quién se avisa al irse (ADR-0005).

#[path = "channel_client/support.rs"]
mod support;
use support::*;

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_is_answered_by_the_errand_and_not_by_the_channel() {
    let held: std::sync::Arc<std::sync::Mutex<Option<ReplyHandle>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&held);

    let channel = AChannel::serving_with(
        ChannelDuty::Serve(NegotiatedCredential::Required(
            ChannelCredential::parse(CREDENTIAL).expect("credencial"),
        )),
        SiteOperations::for_operations(move |url: AfirmaUrl, reply: ReplyHandle| {
            assert_eq!(url.verb(), "selectcert");
            *keeping.lock().expect("el candado") = Some(reply);
        }),
    )
    .await;
    let mut client = channel.a_client().await;

    assert_eq!(client.echo(CREDENTIAL).await.as_deref(), Some("OK"));

    let operation = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");
    client
        .socket
        .send(Message::text(operation))
        .await
        .expect("la operacion deberia salir");

    let waited = tokio::time::timeout(Duration::from_millis(300), client.socket.next()).await;
    assert!(
        waited.is_err(),
        "la operacion no se contesta hasta que lo haga el tramite: {waited:?}"
    );

    let reply = held
        .lock()
        .expect("el candado")
        .take()
        .expect("el tramite recibio el asa");
    let acknowledgement = reply.answer("CANCEL".to_owned());

    let answered = tokio::time::timeout(PATIENCE, client.socket.next())
        .await
        .expect("la respuesta del tramite deberia llegar")
        .expect("hay mensaje")
        .expect("y se lee");
    assert_eq!(answered.into_text().expect("es texto").as_str(), "CANCEL");
    assert!(
        acknowledgement.wait(Duration::from_millis(0)),
        "el cliente ya ha recibido la respuesta: el acuse deberia estar cumplido"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_acknowledgement_is_not_fulfilled_for_a_client_already_gone() {
    let held: std::sync::Arc<std::sync::Mutex<Option<ReplyHandle>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&held);

    let channel = AChannel::serving_with(
        ChannelDuty::Serve(NegotiatedCredential::Required(
            ChannelCredential::parse(CREDENTIAL).expect("credencial"),
        )),
        SiteOperations::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
            *keeping.lock().expect("el candado") = Some(reply);
        }),
    )
    .await;
    let mut client = channel.a_client().await;

    // No se lee la respuesta del eco: queda sin leer en el búfer de recepción del cliente, así
    // que al cerrarlo en caliente el sistema manda un RST en vez de un cierre ordenado.
    client
        .socket
        .send(Message::text(format!("echo=-idsession={CREDENTIAL}@EOF")))
        .await
        .expect("el eco deberia salir");

    let operation = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");
    client
        .socket
        .send(Message::text(operation))
        .await
        .expect("la operacion deberia salir");

    while held.lock().expect("el candado").is_none() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let reply = held
        .lock()
        .expect("el candado")
        .take()
        .expect("el tramite recibio el asa");

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let acknowledgement = reply.answer("CANCEL".to_owned());

    assert!(
        !acknowledgement.wait(Duration::from_millis(300)),
        "el cliente ya se ha ido: el acuse no deberia cumplirse"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn two_operations_over_the_same_socket_get_two_answers() {
    let channel =
        AChannel::serving_with(serving_the_credential(), answering_each_operation()).await;
    let mut client = channel.a_client().await;

    let first = client.say(&an_operation("selectcert")).await;
    let second = client.say(&an_operation("sign")).await;

    assert_eq!(first.as_deref(), Some("contestada:selectcert"));
    assert_eq!(second.as_deref(), Some("contestada:sign"));
    assert!(
        client.is_still_open().await,
        "el canal sigue abierto para la siguiente operacion"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn what_an_operation_leaves_on_the_openssl_error_queue_does_not_break_the_channel() {
    let channel = AChannel::serving_with(
        serving_the_credential(),
        SiteOperations::for_operations(|url: AfirmaUrl, reply: ReplyHandle| {
            let left = openssl::x509::X509::from_pem(b"no es un certificado")
                .expect_err("un PEM roto no se lee");
            for error in left.errors() {
                error.put();
            }
            let _ = reply.answer(format!("contestada:{}", url.verb()));
        }),
    )
    .await;
    let mut client = channel.a_client().await;

    let first = client.say(&an_operation("selectcert")).await;
    let second = client.say(&an_operation("sign")).await;

    assert_eq!(first.as_deref(), Some("contestada:selectcert"));
    assert_eq!(second.as_deref(), Some("contestada:sign"));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_while_another_is_in_flight_is_refused_as_busy() {
    let held: std::sync::Arc<std::sync::Mutex<Option<ReplyHandle>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&held);
    let channel = AChannel::serving_with(
        serving_the_credential(),
        SiteOperations::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
            *keeping.lock().expect("el candado") = Some(reply);
        }),
    )
    .await;
    let mut first = channel.a_client().await;
    let mut second = channel.a_client().await;
    first
        .socket
        .send(Message::text(an_operation("selectcert")))
        .await
        .expect("la operacion deberia salir");
    while held.lock().expect("el candado").is_none() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let refused = second.say(&an_operation("sign")).await;

    assert_eq!(
        refused,
        Some(
            rfirma_lib::site::domain::protocol::WireAnswer::refused(SafCode::CannotOpenSocket)
                .on_the_wire()
        ),
        "no se atienden dos operaciones a la vez"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_first_client_leaving_is_told() {
    let left = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counting = std::sync::Arc::clone(&left);
    let channel = AChannel::serving_with(
        serving_the_credential(),
        answering_each_operation().when_the_first_client_leaves(move || {
            counting.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }),
    )
    .await;
    let mut first = channel.a_client().await;
    assert_eq!(first.echo(CREDENTIAL).await.as_deref(), Some("OK"));
    let mut second = channel.a_client().await;
    assert_eq!(second.echo(CREDENTIAL).await.as_deref(), Some("OK"));

    drop(second);

    assert_eq!(
        counted(&left, 1).await,
        0,
        "un cliente secundario que se va no termina nada"
    );
    assert_eq!(first.echo(CREDENTIAL).await.as_deref(), Some("OK"));

    drop(first);

    assert_eq!(counted(&left, 1).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_client_leaving_mid_operation_is_told() {
    let left = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counting = std::sync::Arc::clone(&left);
    let held: std::sync::Arc<std::sync::Mutex<Option<ReplyHandle>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&held);
    let channel = AChannel::serving_with(
        serving_the_credential(),
        SiteOperations::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
            *keeping.lock().expect("el candado") = Some(reply);
        })
        .when_the_first_client_leaves(move || {
            counting.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }),
    )
    .await;
    let mut client = channel.a_client().await;
    client
        .socket
        .send(Message::text(an_operation("selectcert")))
        .await
        .expect("la operacion deberia salir");
    while held.lock().expect("el candado").is_none() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    client.socket.close(None).await.expect("el cierre sale");

    assert_eq!(counted(&left, 1).await, 1);
}
