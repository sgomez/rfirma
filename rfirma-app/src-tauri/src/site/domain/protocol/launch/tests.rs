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
