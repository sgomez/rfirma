use super::*;

#[test]
fn the_fileid_variant_downloads_and_deciphers_before_delivering() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-1",
            &encrypt(b"contenido-a-firmar", &key),
        )
        .expect("guarda el contenido cifrado");
    servlets.log.lock().expect("el candado").clear();

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), false));

    opened_and_delivered(&relay, &info);

    let (operation, _reply) = spy.take_reply();
    assert_eq!(operation.parameter("dat"), Some("contenido-a-firmar"));
    assert_eq!(operation.parameter("algorithm"), Some("SHA256withRSA"));
    assert_eq!(servlets.log(), vec!["get"]);
}

#[test]
fn the_inline_dat_variant_never_calls_get() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(RelayChannelInfo {
        operation: an_operation("afirma://sign?dat=ya-viene-dentro&algorithm=SHA256withRSA"),
        request: RelayRequest::Inline {
            store_servlet: STORE_SERVLET.to_owned(),
            id: "tx-2".to_owned(),
        },
        key: None,
        active_wait: false,
    });

    opened_and_delivered(&relay, &info);

    let (operation, _reply) = spy.take_reply();
    assert_eq!(operation.parameter("dat"), Some("ya-viene-dentro"));
    assert!(!servlets.log().contains(&"get"));
}

#[test]
fn wait_is_called_before_get_when_the_site_asks_for_it() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", &encrypt(b"contenido", &key))
        .expect("guarda el contenido cifrado");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), true));

    opened_and_delivered(&relay, &info);

    assert_eq!(servlets.log(), vec!["wait", "get"]);
}

#[test]
fn a_successful_upload_reports_no_failure_and_acknowledges_immediately() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(RelayChannelInfo {
        operation: an_operation("afirma://sign?dat=algo&algorithm=SHA256withRSA"),
        request: RelayRequest::Inline {
            store_servlet: STORE_SERVLET.to_owned(),
            id: "tx-3".to_owned(),
        },
        key: None,
        active_wait: false,
    });

    opened_and_delivered(&relay, &info);
    let (_operation, reply) = spy.take_reply();
    let acknowledgement = reply.answer("la-respuesta-cifrada".to_owned());

    assert_eq!(
        servlets.body.retrieve(STORE_SERVLET, "tx-3"),
        Ok("la-respuesta-cifrada".to_owned())
    );
    assert!(spy.failures().is_empty());
    assert!(
        acknowledgement.wait(NO_WAIT),
        "la subida sincrona ya ha terminado: el acuse se cumple al momento"
    );
}

#[test]
fn a_rejected_upload_notifies_without_acknowledging() {
    let servlets = Arc::new(OrderedSpy::that_rejects_the_upload());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(RelayChannelInfo {
        operation: an_operation("afirma://sign?dat=algo&algorithm=SHA256withRSA"),
        request: RelayRequest::Inline {
            store_servlet: STORE_SERVLET.to_owned(),
            id: "tx-4".to_owned(),
        },
        key: None,
        active_wait: false,
    });

    opened_and_delivered(&relay, &info);
    let (_operation, reply) = spy.take_reply();
    let acknowledgement = reply.answer("la-respuesta-cifrada".to_owned());

    let failures = spy.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code(), SafCode::SendingResult);
    assert!(
        !acknowledgement.wait(NO_WAIT),
        "una subida rechazada no entrega la respuesta: el acuse no se cumple"
    );
}

#[test]
fn an_unreachable_servlet_refuses_with_saf_16_without_delivering_anything() {
    let servlets = Arc::new(OrderedSpy {
        body: InMemoryServlets::unreachable(),
        ..OrderedSpy::default()
    });
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(a_key()), false));

    let error = relay
        .open(&info, duty())
        .expect_err("un servlet inalcanzable no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(refusal.code(), SafCode::RecoveringData);
    assert_eq!(
        error.destination(),
        Some((STORE_SERVLET, "tx-1")),
        "el destino ya venia en la url: el fallo al descargar el documento lo lleva consigo"
    );
    assert!(spy.delivered.lock().expect("el candado").is_none());
}

#[test]
fn undecipherable_content_refuses_with_saf_15() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-1", "no-son-bytes-cifrados-validos")
        .expect("guarda basura");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(key), false));

    let error = relay
        .open(&info, duty())
        .expect_err("un contenido indescifrable no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(refusal.code(), SafCode::DecryptingData);
    assert_eq!(error.destination(), Some((STORE_SERVLET, "tx-1")));
}

#[test]
fn a_refuse_duty_leaves_the_upload_as_a_pending_delivery_instead_of_running_it_inside_open() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(a_key()), true));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    let mut channel = relay
        .open(&info, ChannelDuty::Refuse(answer))
        .expect("abre con la entrega pendiente, sin subir todavia");

    assert_eq!(channel.arrival_mode(), ArrivalMode::Immediate);
    assert!(
        servlets.log().is_empty(),
        "abrir el canal no sube nada: la entrega queda pendiente de que la dispare quien atiende"
    );

    let handed_out = channel
        .take_delivery()
        .expect("una llegada inmediata siempre trae entrega")
        .now();

    assert!(handed_out, "el servlet acepto la subida");
    assert_eq!(servlets.log(), vec!["put"]);
    assert!(spy.delivered.lock().expect("el candado").is_none());
    assert!(spy.failures().is_empty());
}

#[test]
fn a_refuse_duty_delivery_that_fails_to_upload_notifies_the_failure() {
    let servlets = Arc::new(OrderedSpy::that_rejects_the_upload());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(RETRIEVE_SERVLET, Some(a_key()), false));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    let mut channel = relay
        .open(&info, ChannelDuty::Refuse(answer))
        .expect("abre con la entrega pendiente");

    let handed_out = channel
        .take_delivery()
        .expect("una llegada inmediata siempre trae entrega")
        .now();

    assert!(!handed_out, "el servlet rechazo la subida");
    let failures = spy.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code(), SafCode::SendingResult);
}

#[test]
fn the_fileid_variant_with_gzip_deciphers_then_delivers_for_decompression() {
    let key = a_key();
    let compressed = {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(b"%PDF-1.7\nrelay-gzip")
            .expect("comprime");
        encoder.finish().expect("termina")
    };
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-gzip-1",
            &encrypt(
                base64::engine::general_purpose::URL_SAFE
                    .encode(&compressed)
                    .as_bytes(),
                &key,
            ),
        )
        .expect("guarda el contenido cifrado");
    servlets.log.lock().expect("el candado").clear();

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let mut info_data = a_fileid_info(RETRIEVE_SERVLET, Some(key), false);
    info_data.request = RelayRequest::DataByFileId {
        store_servlet: STORE_SERVLET.to_owned(),
        id: "tx-1".to_owned(),
        fileid: "fileid-gzip-1".to_owned(),
        retrieve_servlet: RETRIEVE_SERVLET.to_owned(),
    };
    info_data.operation =
        an_operation("afirma://sign?op=sign&format=PAdES&algorithm=SHA256withRSA&gzip=true");
    let info = ChannelLocation::Relay(info_data);

    opened_and_delivered(&relay, &info);

    let (operation, _reply) = spy.take_reply();
    assert_eq!(operation.parameter("gzip"), Some("true"));
    let SiteOperation::Sign(request) = read_operation(&operation).expect("lee operacion") else {
        panic!("esperaba sign");
    };
    assert_eq!(request.document(), b"%PDF-1.7\nrelay-gzip");
    assert_eq!(servlets.log(), vec!["get"]);
}
