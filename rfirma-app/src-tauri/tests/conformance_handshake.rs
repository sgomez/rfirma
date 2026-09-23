//! Negociación del protocolo entre el cliente publicado y el canal que abre rFirma, versión a versión.

#[allow(dead_code, unused_imports)]
mod support;

use support::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_url_the_published_client_builds_is_the_one_rfirma_reads() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 4 se habla aqui");

    assert_eq!(
        PROTOCOL_VERSION, THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS,
        "rfirma implementa la version que el cliente publicado envia"
    );
    let ChannelLocation::Drawn(ports) = launch.location() else {
        panic!(
            "el cliente publicado sortea puertos: {:?}",
            launch.location()
        );
    };
    assert_eq!(
        ports.len(),
        3,
        "el cliente publicado sortea tres puertos, y llegaron {ports:?}"
    );
    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado trae credencial de canal");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unsupported_version_reaches_the_error_callback_of_the_published_client() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();

    let unsupported = url.replace(
        &format!("&v={THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS}"),
        "&v=99",
    );
    let refusal = LaunchRequest::parse(&unsupported).expect_err("la version 99 no se habla aqui");
    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);

    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let _channel =
        the_channel_on_one_of(&parsed, &material, ChannelDuty::Refuse(refusal.answer())).await;

    let verdict = client.next_event();

    assert_eq!(
        verdict.name(),
        "error",
        "el trámite tiene que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("type"),
        "java.lang.InterruptedException",
        "lo medido contra el tag v1.9.2: el cierre del canal es lo que el \
         cliente publicado convierte en error, no el `SAF_21` que le contestamos"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_third_protocol_connects_to_the_port_rfirma_was_told_to_open() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Third);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 3 se habla aqui, sin puertos");

    assert_eq!(
        parsed.parameter("idsession").map(str::len),
        Some(20),
        "el cliente publicado, aunque hable la version 3, sigue mandando idsession"
    );
    assert_eq!(
        launch.credential(),
        &NegotiatedCredential::Absent,
        "la version 3 no exige el idsession que manda la sede"
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        no_operations(),
    )
    .await;
    assert_eq!(
        Some(channel.port()),
        client.third_port,
        "el canal se abre en el puerto que se le dio al cliente publicado"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_message_reveals_the_window_before_the_operation() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();
    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("version 4");

    let sequence = Arc::new(Mutex::new(Vec::new()));
    let seq_arr = Arc::clone(&sequence);
    let seq_del = Arc::clone(&sequence);

    let (delivered, waited) = tokio::sync::oneshot::channel();
    let delivered = Arc::new(Mutex::new(Some(delivered)));

    let inbox = SiteOperations::of(
        move || {
            seq_arr.lock().unwrap().push("ventana".to_owned());
        },
        move |_url, reply| {
            seq_del.lock().unwrap().push("operacion".to_owned());
            if let Some(delivered) = delivered.lock().unwrap().take() {
                let _ = delivered.send(());
            }
            reply.answer(r#"{"state":"ok"}"#.to_owned());
        },
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        inbox,
    )
    .await;

    let _ = tokio::time::timeout(Duration::from_secs(10), waited).await;

    let steps = sequence.lock().unwrap().clone();
    assert_eq!(
        steps,
        vec!["ventana".to_owned(), "operacion".to_owned()],
        "el eco o mensaje inicial revela la ventana antes de la operacion"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_message_over_the_third_protocol_reveals_the_window_before_the_operation() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Third);
    let url = client.the_launch_url();
    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("version 3");

    let sequence = Arc::new(Mutex::new(Vec::new()));
    let seq_arr = Arc::clone(&sequence);
    let seq_del = Arc::clone(&sequence);

    let (delivered, waited) = tokio::sync::oneshot::channel();
    let delivered = Arc::new(Mutex::new(Some(delivered)));

    let inbox = SiteOperations::of(
        move || {
            seq_arr.lock().unwrap().push("ventana".to_owned());
        },
        move |_url, reply| {
            seq_del.lock().unwrap().push("operacion".to_owned());
            if let Some(delivered) = delivered.lock().unwrap().take() {
                let _ = delivered.send(());
            }
            reply.answer(r#"{"state":"ok"}"#.to_owned());
        },
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        inbox,
    )
    .await;

    let _ = tokio::time::timeout(Duration::from_secs(10), waited).await;

    let steps = sequence.lock().unwrap().clone();
    assert_eq!(
        steps,
        vec!["ventana".to_owned(), "operacion".to_owned()],
        "el primer mensaje en v3 revela la ventana antes de la operacion"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_websocket_the_published_client_falls_back_to_service_v1() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Service);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 1 de 'service' se habla aqui");

    assert_eq!(
        launch.version(),
        THE_SERVICE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS,
        "sin WebSocket el cliente publicado habla la version 1 de 'service'"
    );
    let ChannelLocation::Service(ports) = launch.location() else {
        panic!(
            "el cliente publicado sortea puertos tambien para 'service': {:?}",
            launch.location()
        );
    };
    assert!(
        !ports.is_empty(),
        "'service' sortea puertos igual que la version 4 de 'websocket'"
    );
    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado manda idsession tambien en 'service'");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos, igual que en 'websocket'"
    );
}
