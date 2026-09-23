use super::*;

#[test]
fn the_parameters_variant_reads_the_operation_and_its_servlets_from_the_recovered_xml() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-1",
            &encrypt(
                &a_parameters_xml(&[
                    ("op", "sign"),
                    ("format", "PAdES"),
                    ("algorithm", "SHA256withRSA"),
                    ("stservlet", "https://sede.example/store"),
                    ("id", "txdelxml"),
                    ("dat", "JVBERi0xLjc"),
                ]),
                &key,
            ),
        )
        .expect("guarda el XML de parametros cifrado");
    servlets.log.lock().expect("el candado").clear();

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-1", Some(key)));

    opened_and_delivered(&relay, &info);

    let (operation, reply) = spy.take_reply();
    assert_eq!(operation.verb(), "sign");
    assert_eq!(operation.parameter("format"), Some("PAdES"));
    assert_eq!(operation.parameter("dat"), Some("JVBERi0xLjc"));

    reply.answer("la-respuesta-cifrada".to_owned());
    assert_eq!(
        servlets
            .body
            .retrieve("https://sede.example/store", "txdelxml"),
        Ok("la-respuesta-cifrada".to_owned()),
        "la respuesta sube al 'stservlet' y con el 'id' que venian dentro del XML"
    );
}

#[test]
fn the_parameters_variant_applies_gzip_after_deciphering() {
    let key = a_key();
    let compressed = {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(b"%PDF-1.7\nparametros-gzip")
            .expect("comprime");
        encoder.finish().expect("termina")
    };
    let document = base64::engine::general_purpose::URL_SAFE.encode(&compressed);
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-gzip",
            &encrypt(
                &a_parameters_xml(&[
                    ("op", "sign"),
                    ("format", "PAdES"),
                    ("algorithm", "SHA256withRSA"),
                    ("gzip", "true"),
                    ("stservlet", STORE_SERVLET),
                    ("id", "txgzip"),
                    ("dat", &document),
                ]),
                &key,
            ),
        )
        .expect("guarda el XML de parametros cifrado");

    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-gzip", Some(key)));

    opened_and_delivered(&relay, &info);

    let (operation, _reply) = spy.take_reply();
    let SiteOperation::Sign(request) = read_operation(&operation).expect("lee operacion") else {
        panic!("esperaba sign");
    };
    assert_eq!(request.document(), b"%PDF-1.7\nparametros-gzip");
}

#[test]
fn an_unrecoverable_parameters_xml_refuses_with_the_same_code_as_a_document() {
    let servlets = Arc::new(OrderedSpy {
        body: InMemoryServlets::unreachable(),
        ..OrderedSpy::default()
    });
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-2", Some(a_key())));

    let error = relay
        .open(&info, duty())
        .expect_err("un servlet inalcanzable no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(refusal.code(), SafCode::RecoveringData);
    assert!(spy.delivered.lock().expect("el candado").is_none());
}

#[test]
fn an_undecipherable_parameters_xml_refuses_with_saf_15() {
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(STORE_SERVLET, "fileid-params-3", "no-son-bytes-cifrados")
        .expect("guarda basura");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-3", Some(a_key())));

    let error = relay
        .open(&info, duty())
        .expect_err("un contenido indescifrable no abre");

    assert_eq!(
        error
            .refusal()
            .expect("trae su propio rechazo clasificado")
            .code(),
        SafCode::DecryptingData
    );
}

#[test]
fn a_servlet_error_response_refuses_with_saf_16_not_decryption_failure() {
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-err",
            "ERR-06:=El identificador para los datos es inválido ('fileid-params-err')\n",
        )
        .expect("guarda error");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-err", Some(a_key())));

    let error = relay
        .open(&info, duty())
        .expect_err("una respuesta con error de servlet no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(
        error.detail(),
        "ERR-06:=El identificador para los datos es inválido ('fileid-params-err')"
    );
    assert_eq!(refusal.code(), SafCode::RecoveringData);
}

#[test]
fn an_illegible_parameters_xml_refuses_as_a_parameters_problem() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-4",
            &encrypt(b"esto no es el XML de parametros", &key),
        )
        .expect("guarda algo que no es el XML");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-4", Some(key)));

    let error = relay
        .open(&info, duty())
        .expect_err("un XML de parametros ilegible no abre");

    assert_eq!(
        error
            .refusal()
            .expect("trae su propio rechazo clasificado")
            .code(),
        SafCode::Params
    );
}

#[test]
fn a_parameters_xml_without_stservlet_refuses_as_a_parameters_problem() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-5",
            &encrypt(
                &a_parameters_xml(&[("op", "sign"), ("id", "txsinservlet")]),
                &key,
            ),
        )
        .expect("guarda el XML sin 'stservlet'");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-5", Some(key)));

    let error = relay
        .open(&info, duty())
        .expect_err("sin 'stservlet' no hay adonde contestar");

    assert_eq!(
        error
            .refusal()
            .expect("trae su propio rechazo clasificado")
            .code(),
        SafCode::Params
    );
}

#[test]
fn a_parameters_xml_with_a_too_long_id_refuses_as_a_parameters_problem() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-7",
            &encrypt(
                &a_parameters_xml(&[
                    ("op", "sign"),
                    ("stservlet", STORE_SERVLET),
                    ("id", "abcdefghijklmnopqrstu"),
                ]),
                &key,
            ),
        )
        .expect("guarda el XML con un 'id' de veintiun caracteres");

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-7", Some(key)));

    let error = relay
        .open(&info, duty())
        .expect_err("el 'id' del XML pasa por la misma guarda que el de la URL");

    assert_eq!(
        error
            .refusal()
            .expect("trae su propio rechazo clasificado")
            .code(),
        SafCode::Params
    );
}

#[test]
fn the_parameters_variant_waits_when_the_recovered_xml_asks_for_it() {
    let key = a_key();
    let servlets = Arc::new(OrderedSpy::default());
    servlets
        .store(
            STORE_SERVLET,
            "fileid-params-6",
            &encrypt(
                &a_parameters_xml(&[
                    ("op", "selectcert"),
                    ("aw", "true"),
                    ("stservlet", STORE_SERVLET),
                    ("id", "txespera"),
                ]),
                &key,
            ),
        )
        .expect("guarda el XML de parametros cifrado");
    servlets.log.lock().expect("el candado").clear();

    let (relay, _spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-6", Some(key)));

    opened_and_delivered(&relay, &info);

    assert_eq!(
        servlets.log(),
        vec!["get", "wait"],
        "la espera activa la pide el XML, y por eso llega despues de recuperarlo"
    );
}

#[test]
fn a_refuse_duty_without_a_store_target_does_not_open_the_channel() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_parameters_info("fileid-params-7", Some(a_key())));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    let error = relay
        .open(&info, ChannelDuty::Refuse(answer))
        .expect_err("todavia no se sabe adonde subir la respuesta");

    assert_eq!(error.situation(), Situation::Relay);
    assert!(servlets.log().is_empty());
    assert!(spy.failures().is_empty());
}
