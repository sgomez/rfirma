use super::*;
use crate::site::domain::channel::ChannelLocation;

/// La invocación que manda el `autoscript.js` publicado, tal cual.
const PUBLISHED: &str =
    "afirma://websocket?ports=49152,50001,60123&v=4&jvc=3&idsession=BQXf7mJ2Kd9pLzR3tYvW";

#[test]
fn the_launch_invocation_the_published_client_sends_is_read_whole() {
    let request = LaunchRequest::parse(PUBLISHED).expect("la invocacion publicada deberia valer");

    assert_eq!(
        request.location(),
        &ChannelLocation::Drawn(vec![49152, 50001, 60123])
    );
    assert_eq!(
        request.credential(),
        &NegotiatedCredential::Required(
            ChannelCredential::parse("BQXf7mJ2Kd9pLzR3tYvW").expect("la credencial es buena")
        )
    );
}

#[test]
fn the_javascript_version_code_is_ignored_on_purpose() {
    for jvc in ["jvc=3", "jvc=0", "jvc=noesunnumero", ""] {
        let url = format!("afirma://websocket?ports=49152&v=4&{jvc}&idsession=abc");
        assert!(LaunchRequest::parse(&url).is_ok(), "con {jvc}");
    }
}

#[test]
fn an_absent_version_is_version_one_and_therefore_unsupported() {
    let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&idsession=abc")
        .expect_err("sin 'v' la version es la 1");

    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
    assert!(refusal.detail().contains("version de protocolo 1"));
}

#[test]
fn the_third_protocol_opens_on_the_fixed_port_without_ports() {
    let request = LaunchRequest::parse("afirma://websocket?v=3&idsession=abc")
        .expect("el protocolo 3 no trae puertos");

    assert_eq!(
        request.location(),
        &ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL)
    );
    assert_eq!(
        request.credential(),
        &NegotiatedCredential::Required(ChannelCredential::parse("abc").expect("vale"))
    );
}

#[test]
fn the_third_protocol_without_idsession_negotiates_no_credential() {
    let request =
        LaunchRequest::parse("afirma://websocket?v=3").expect("el protocolo 3 no exige idsession");

    assert_eq!(request.credential(), &NegotiatedCredential::Absent);
}

#[test]
fn the_third_protocol_still_rejects_a_malformed_credential() {
    let refusal = LaunchRequest::parse("afirma://websocket?v=3&idsession=abc-def")
        .expect_err("una credencial mal formada se rechaza tambien en el protocolo 3");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn the_third_protocol_ignores_ports_if_the_site_sent_any() {
    let request = LaunchRequest::parse("afirma://websocket?ports=49152&v=3&idsession=abc")
        .expect("el protocolo 3 abre siempre en el puerto fijo");

    assert_eq!(
        request.location(),
        &ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL)
    );
}

#[test]
fn a_version_that_is_neither_three_nor_four_is_unsupported() {
    let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=2&idsession=abc")
        .expect_err("solo se hablan la 3 y la 4");

    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
}

#[test]
fn a_version_that_is_not_a_number_falls_into_the_same_hole_as_an_absent_one() {
    let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=4.1&idsession=abc")
        .expect_err("la 4.1 no existe y no hay forma de expresarla");

    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
    assert!(refusal.detail().contains("version de protocolo 1"));
}

#[test]
fn a_version_written_with_spaces_is_trimmed_like_in_the_original() {
    let request = LaunchRequest::parse("afirma://websocket?ports=49152&v=%204%20&idsession=abc")
        .expect("el original hace trim antes de parsear");

    assert_eq!(request.location(), &ChannelLocation::Drawn(vec![49152]));
}

#[test]
fn a_malformed_channel_credential_is_refused_instead_of_nulled() {
    for idsession in [
        "idsession=",
        "idsession=abc-def",
        "idsession=abc def",
        "idsession=ñ",
    ] {
        let url = format!("afirma://websocket?ports=49152&v=4&{idsession}");
        let refusal = LaunchRequest::parse(&url)
            .expect_err("un idsession malo abriria un canal sin cerradura");

        assert_eq!(refusal.code(), SafCode::Params, "con {idsession}");
    }
}

#[test]
fn an_absent_channel_credential_is_refused_in_the_fourth_protocol() {
    let refusal = LaunchRequest::parse("afirma://websocket?ports=49152&v=4")
        .expect_err("en el protocolo 4 no hay canal sin credencial");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_short_credential_is_accepted_because_the_original_has_no_floor() {
    let request = LaunchRequest::parse("afirma://websocket?ports=49152&v=4&idsession=a")
        .expect("un solo caracter esta bien formado");

    assert_eq!(
        request.credential(),
        &NegotiatedCredential::Required(ChannelCredential::parse("a").expect("vale"))
    );
}

#[test]
fn the_drawn_ports_are_readable_from_a_launch_that_is_refused() {
    let url = AfirmaUrl::parse("afirma://websocket?ports=54001,54002&v=4&idsession=malformado!")
        .expect("es una URL del protocolo");

    assert!(
        LaunchRequest::from_url(&url).is_err(),
        "la credencial no vale"
    );
    assert_eq!(drawn_ports(&url), vec![54001, 54002]);
}

#[test]
fn a_launch_without_readable_ports_draws_none() {
    let without = AfirmaUrl::parse("afirma://websocket?v=3").expect("es una URL del protocolo");
    let unreadable =
        AfirmaUrl::parse("afirma://websocket?ports=setenta&v=4").expect("es una URL del protocolo");

    assert!(drawn_ports(&without).is_empty());
    assert!(drawn_ports(&unreadable).is_empty());
}

#[test]
fn the_ports_keep_the_order_the_site_drew_them_in() {
    let request =
        LaunchRequest::parse("afirma://websocket?ports=60123,49152,50001&v=4&idsession=abc")
            .expect("parsea");

    assert_eq!(
        request.location(),
        &ChannelLocation::Drawn(vec![60123, 49152, 50001])
    );
}

#[test]
fn a_negative_port_is_taken_by_its_absolute_value_like_in_the_original() {
    let request = LaunchRequest::parse("afirma://websocket?ports=-49152&v=4&idsession=abc")
        .expect("el original hace Math.abs");

    assert_eq!(request.location(), &ChannelLocation::Drawn(vec![49152]));
}

#[test]
fn ports_that_cannot_be_bound_are_a_parameter_error() {
    for ports in [
        "ports=",
        "ports=abc",
        "ports=49152,abc",
        "ports=0",
        "ports=70000",
        "ports=-9223372036854775808",
    ] {
        let url = format!("afirma://websocket?{ports}&v=4&idsession=abc");
        let refusal = LaunchRequest::parse(&url).expect_err("no es un puerto");

        assert_eq!(refusal.code(), SafCode::Params, "con {ports}");
    }
}

#[test]
fn the_fourth_protocol_without_ports_does_not_fall_back_to_the_fixed_port() {
    let refusal = LaunchRequest::parse("afirma://websocket?v=4&idsession=abc")
        .expect_err("el camino sin puertos es el del protocolo 3");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn only_the_websocket_verb_opens_a_channel() {
    let refusal = LaunchRequest::parse("afirma://sign?ports=49152&v=4&idsession=abc")
        .expect_err("la invocacion de arranque es 'websocket'");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_refusal_location_falls_back_to_the_fixed_port_when_the_site_declared_the_third_protocol() {
    let url = AfirmaUrl::parse("afirma://websocket?v=3&idsession=abc-def")
        .expect("es una URL del protocolo");
    assert!(
        LaunchRequest::from_url(&url).is_err(),
        "el idsession no vale"
    );

    assert_eq!(
        location_for_a_refusal(&url),
        Some(ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL))
    );
}

#[test]
fn a_refusal_location_prefers_drawn_ports_when_the_site_sent_any() {
    let url =
        AfirmaUrl::parse("afirma://websocket?ports=54001&v=2").expect("es una URL del protocolo");

    assert_eq!(
        location_for_a_refusal(&url),
        Some(ChannelLocation::Drawn(vec![54001]))
    );
}

#[test]
fn a_refusal_location_is_none_without_ports_nor_the_third_protocol() {
    let url = AfirmaUrl::parse("afirma://websocket?v=99").expect("es una URL del protocolo");

    assert_eq!(location_for_a_refusal(&url), None);
}

#[test]
fn a_refusal_location_of_a_relay_launch_with_stservlet_and_id_is_its_own_destination() {
    let url = AfirmaUrl::parse(
        "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.example/store\
         &id=tx2",
    )
    .expect("es una URL de servidor intermedio");

    assert_eq!(
        location_for_a_refusal(&url),
        Some(ChannelLocation::Relay(RelayChannelInfo {
            operation: url.clone(),
            request: RelayRequest::Inline {
                store_servlet: "https://relay.example/store".to_owned(),
                id: "tx2".to_owned(),
            },
            key: None,
            active_wait: false,
        }))
    );
}

#[test]
fn a_refusal_location_of_a_relay_launch_without_stservlet_in_the_url_is_none() {
    let url = AfirmaUrl::parse(PUBLISHED_PARAMETERS_BY_FILEID)
        .expect("es una URL de servidor intermedio con solo fileid");

    assert_eq!(
        location_for_a_refusal(&url),
        None,
        "el destino solo esta dentro del XML de parametros: no se descarga para un rechazo"
    );
}

/// La invocación que manda la sede que fuerza servidor intermedio con un dato de tamaño real:
/// `buildUrlWithoutData` solo añade `fileid`, `rtservlet` y `key` (`autoscript.js:4489`).
const PUBLISHED_PARAMETERS_BY_FILEID: &str =
    "afirma://sign?jvc=3&fileid=KDpNbXwqTFY5N0F0djJZ&rtservlet=https://sede.example/afirma-signature-retriever/RetrieveService&key=12345678";

#[test]
fn the_fileid_only_invocation_the_published_client_sends_is_read_as_a_relay_launch() {
    let request = LaunchRequest::parse(PUBLISHED_PARAMETERS_BY_FILEID)
        .expect("la forma fileid-only es un arranque de servidor intermedio");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion que recupera sus parametros negocia canal de servidor intermedio");
    };
    assert_eq!(
        info.request,
        RelayRequest::ParametersByFileId {
            fileid: "KDpNbXwqTFY5N0F0djJZ".to_owned(),
            retrieve_servlet: "https://sede.example/afirma-signature-retriever/RetrieveService"
                .to_owned(),
        }
    );
    assert!(info.key.is_some());
}

#[test]
fn the_fileid_only_invocation_does_not_declare_where_to_upload_the_answer_yet() {
    let request =
        LaunchRequest::parse(PUBLISHED_PARAMETERS_BY_FILEID).expect("la forma fileid-only vale");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion que recupera sus parametros negocia canal de servidor intermedio");
    };
    assert_eq!(info.request.store_target(), None);
}

#[test]
fn a_relay_launch_with_fileid_needs_rtservlet_and_stores_the_channel_info() {
    let request = LaunchRequest::parse(
        "afirma://sign?algorithm=SHA256withRSA&fileid=abc123&rtservlet=https://relay.example/retrieve\
         &stservlet=https://relay.example/store&key=12345678&id=tx1",
    )
    .expect("la variante fileid deberia valer");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion con servlet negocia canal de servidor intermedio");
    };
    assert_eq!(info.operation.verb(), "sign");
    assert_eq!(
        info.request,
        RelayRequest::DataByFileId {
            store_servlet: "https://relay.example/store".to_owned(),
            id: "tx1".to_owned(),
            fileid: "abc123".to_owned(),
            retrieve_servlet: "https://relay.example/retrieve".to_owned(),
        }
    );
    assert!(info.key.is_some());
    assert!(!info.active_wait);
}

#[test]
fn a_relay_launch_with_inline_dat_does_not_need_rtservlet_nor_key() {
    let request = LaunchRequest::parse(
        "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.example/store\
         &id=tx2",
    )
    .expect("la variante dat inline deberia valer");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion con servlet negocia canal de servidor intermedio");
    };
    assert_eq!(
        info.request,
        RelayRequest::Inline {
            store_servlet: "https://relay.example/store".to_owned(),
            id: "tx2".to_owned(),
        }
    );
    assert!(info.key.is_none());
}

#[test]
fn a_relay_launch_without_stservlet_and_without_rtservlet_is_refused() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?algorithm=SHA256withRSA&fileid=abc&id=tx3&key=12345678&aw=true",
    )
    .expect_err("sin stservlet ni rtservlet no hay ni respuesta que subir ni parametros que leer");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_relay_launch_with_fileid_but_no_rtservlet_is_refused() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?algorithm=SHA256withRSA&fileid=abc&stservlet=https://relay.example/store\
         &id=tx4",
    )
    .expect_err("sin rtservlet no se puede recuperar el fileid");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_relay_launch_with_neither_dat_nor_fileid_is_refused() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?algorithm=SHA256withRSA&stservlet=https://relay.example/store&id=tx5",
    )
    .expect_err("sin datos que operar no hay nada que hacer");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_relay_launch_reads_the_active_wait_flag() {
    let request = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx6&aw=true",
    )
    .expect("la operacion deberia valer");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion con servlet negocia canal de servidor intermedio");
    };
    assert!(info.active_wait);
}

/// La invocación de `service` que manda el `autoscript.js` publicado sin WebSocket
/// (`autoscript.js:2929`).
const PUBLISHED_SERVICE: &str =
    "afirma://service?ports=49152,50001,60123&v=1&jvc=3&idsession=BQXf7mJ2Kd9pLzR3tYvW";

#[test]
fn the_service_invocation_the_published_client_sends_is_read_whole() {
    let request =
        LaunchRequest::parse(PUBLISHED_SERVICE).expect("la invocacion publicada deberia valer");

    assert_eq!(
        request.location(),
        &ChannelLocation::Service(vec![49152, 50001, 60123])
    );
    assert_eq!(
        request.credential(),
        &NegotiatedCredential::Required(
            ChannelCredential::parse("BQXf7mJ2Kd9pLzR3tYvW").expect("la credencial es buena")
        )
    );
}

#[test]
fn a_service_launch_without_v_defaults_to_the_first_protocol() {
    let request = LaunchRequest::parse("afirma://service?ports=49152&idsession=abc")
        .expect("sin 'v' la version es la 1");

    assert_eq!(request.version(), 1);
}

#[test]
fn a_service_launch_accepts_versions_one_two_and_three() {
    for v in ["1", "2", "3"] {
        let url = format!("afirma://service?ports=49152&v={v}&idsession=abc");
        assert!(LaunchRequest::parse(&url).is_ok(), "con v={v}");
    }
}

#[test]
fn a_service_launch_rejects_versions_outside_one_two_and_three() {
    let refusal = LaunchRequest::parse("afirma://service?ports=49152&v=4&idsession=abc")
        .expect_err("'service' no habla la version 4");

    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
}

#[test]
fn a_service_launch_without_ports_is_a_parameter_error() {
    let refusal = LaunchRequest::parse("afirma://service?v=1&idsession=abc")
        .expect_err("'service' siempre exige 'ports'");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn a_service_launch_without_idsession_negotiates_no_credential_regardless_of_version() {
    for v in ["1", "2", "3"] {
        let url = format!("afirma://service?ports=49152&v={v}");
        let request = LaunchRequest::parse(&url).expect("sin 'idsession' vale igualmente");

        assert_eq!(
            request.credential(),
            &NegotiatedCredential::Absent,
            "con v={v}"
        );
    }
}

#[test]
fn a_service_launch_with_idsession_requires_it() {
    let request =
        LaunchRequest::parse("afirma://service?ports=49152&v=1&idsession=abc").expect("vale");

    assert_eq!(
        request.credential(),
        &NegotiatedCredential::Required(ChannelCredential::parse("abc").expect("vale"))
    );
}

#[test]
fn a_refusal_location_of_a_service_launch_uses_the_service_variant() {
    let url = AfirmaUrl::parse("afirma://service?ports=49152,50001&v=1").unwrap();

    assert_eq!(
        location_for_a_refusal(&url),
        Some(ChannelLocation::Service(vec![49152, 50001]))
    );
}

#[test]
fn a_refusal_location_of_a_service_launch_without_ports_is_none() {
    let url = AfirmaUrl::parse("afirma://service?v=1").unwrap();

    assert_eq!(location_for_a_refusal(&url), None);
}

#[test]
fn an_identifier_longer_than_twenty_characters_is_refused_naming_it() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=abcdefghij0123456789X",
    )
    .expect_err("el original exige veinte caracteres como mucho");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Identifier));
}

#[test]
fn an_identifier_that_is_not_alphanumeric_is_refused() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx%5F1",
    )
    .expect_err("el identificador acaba siendo un nombre de fichero");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Identifier));
}

#[test]
fn a_fileid_carries_the_same_two_guards_as_an_identifier() {
    let refusal =
        LaunchRequest::parse("afirma://sign?fileid=ab*c&rtservlet=https://relay.example/retrieve")
            .expect_err("el 'fileid' hace de identificador en las cinco clases del original");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::FileId));
}

#[test]
fn a_store_servlet_on_a_local_address_is_refused_as_a_local_access() {
    for host in ["localhost", "127.0.0.1"] {
        let refusal = LaunchRequest::parse(&format!(
            "afirma://sign?dat=ZmlybWFkbw&stservlet=https://{host}/store&id=tx1"
        ))
        .expect_err("un host local sale con su propio codigo");

        assert_eq!(refusal.code(), SafCode::LocalAccessBlocked, "con {host}");
    }
}

#[test]
fn a_retrieve_servlet_on_a_local_address_is_refused_as_a_local_access() {
    let refusal =
        LaunchRequest::parse("afirma://sign?fileid=abc123&rtservlet=http://127.0.0.1/retrieve")
            .expect_err("un host local sale con su propio codigo");

    assert_eq!(refusal.code(), SafCode::LocalAccessBlocked);
}

#[test]
fn a_servlet_url_with_its_own_parameters_is_refused_naming_it() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store%3Fop%3Dput&id=tx1",
    )
    .expect_err("el original prohibe '?' y '=' en la url del servlet");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::StoreServlet));
}

#[test]
fn a_servlet_url_with_an_unsupported_scheme_is_refused_naming_it() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=ftp://relay.example/store&id=tx1",
    )
    .expect_err("solo se admiten 'http' y 'https'");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::StoreServlet));
}

#[test]
fn an_http_servlet_url_is_read_like_the_original_reads_it() {
    assert!(LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=http://relay.example/store&id=tx1"
    )
    .is_ok());
}

#[test]
fn a_cipher_key_of_the_wrong_length_is_refused_naming_the_key() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx1&key=1234567",
    )
    .expect_err("el original exige ocho caracteres");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::CipherKey));
}

#[test]
fn the_active_wait_flag_is_read_like_boolean_parse_boolean() {
    let asked = |value: &str| {
        asks_for_active_wait(
            &AfirmaUrl::parse(&format!("afirma://sign?aw={value}")).expect("es una url"),
        )
    };

    assert!(asked("true"));
    assert!(asked("TRUE"));
    assert!(!asked("1"));
    assert!(!asked("yes"));
    assert!(!asked("false"));
    assert!(!asked(""));
}

#[test]
fn the_version_of_a_relay_launch_comes_from_ver_and_not_from_a_constant() {
    let declared = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx1&ver=3",
    )
    .expect("la invocacion por servidor intermedio deberia valer");

    assert_eq!(declared.version(), 3);

    let silent = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx1",
    )
    .expect("la invocacion por servidor intermedio deberia valer");

    assert_eq!(silent.version(), 0, "sin 'ver' la operacion no exige nada");
}

#[test]
fn a_relay_launch_that_demands_a_protocol_version_not_spoken_here_is_refused() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx1&ver=5",
    )
    .expect_err("aqui no se habla la version 5 del protocolo");

    assert_eq!(refusal.code(), SafCode::MinimumVersionNonSatisfied);
    assert_eq!(
        refusal.situation(),
        RefusalSituation::UnsupportedProtocolVersion
    );
}

#[test]
fn the_launch_version_still_rules_the_channel_that_is_already_open() {
    let request = LaunchRequest::parse(&format!("{PUBLISHED}&ver=5"))
        .expect("el arranque declara su version en 'v' y es la que manda");

    assert_eq!(request.version(), PROTOCOL_VERSION);
}
