use super::*;

#[tokio::test(start_paused = true)]
async fn when_active_wait_is_requested_it_pulses_wait_periodically() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    let _channel = opened_and_delivered(&relay, &info);
    tokio::task::yield_now().await;

    assert_eq!(servlets.log(), vec!["wait", "get"]);

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait", "wait"]);
}

#[tokio::test(start_paused = true)]
async fn when_reply_answers_the_active_wait_heartbeat_stops_before_upload() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    let _channel = opened_and_delivered(&relay, &info);
    let (_url, reply) = spy.take_reply();
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);

    reply.answer("resultado_final".into());
    assert_eq!(servlets.log(), vec!["wait", "get", "wait", "put"]);

    tokio::time::advance(Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait", "put"]);
}

#[tokio::test(start_paused = true)]
async fn when_channel_is_closed_the_active_wait_heartbeat_stops() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    let channel = opened_and_delivered(&relay, &info);
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);

    channel.close();

    tokio::time::advance(Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);
}

#[tokio::test(start_paused = true)]
async fn when_active_wait_is_false_no_wait_is_sent_and_no_heartbeat_runs() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), false));

    let _channel = opened_and_delivered(&relay, &info);
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["get"]);

    tokio::time::advance(Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["get"]);
}

#[tokio::test(start_paused = true)]
async fn parameters_variant_with_active_wait_pulses_periodically() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-aw",
            &encrypt(
                &a_parameters_xml(&[
                    ("op", "sign"),
                    ("aw", "true"),
                    ("stservlet", STORE_SERVLET),
                    ("id", "txpulse"),
                ]),
                &key,
            ),
        )
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-aw", Some(key)));

    let _channel = opened_and_delivered(&relay, &info);
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["get", "wait"]);

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["get", "wait", "wait"]);
}

#[tokio::test(start_paused = true)]
async fn when_handles_are_dropped_without_answering_the_heartbeat_stops() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    let channel = opened_and_delivered(&relay, &info);
    let (_url, reply) = spy.take_reply();
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::task::yield_now().await;
    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);

    drop(channel);
    drop(reply);
    tokio::task::yield_now().await;

    tokio::time::advance(Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    assert_eq!(
        servlets.log(),
        vec!["wait", "get", "wait"],
        "el latido debe detenerse si los asideros se descartan sin responder"
    );
}

#[test]
fn the_heartbeat_pulses_when_the_channel_opens_outside_the_runtime() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("el runtime de la prueba arranca");
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda");
    servlets.log.lock().expect("el candado").clear();
    let (relay, _spy) = a_relay_on(Arc::clone(&servlets), runtime.handle().clone());
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    let _channel = opened_and_delivered(&relay, &info);
    runtime.block_on(async {
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(10)).await;
        tokio::task::yield_now().await;
    });

    assert_eq!(servlets.log(), vec!["wait", "get", "wait"]);
}
