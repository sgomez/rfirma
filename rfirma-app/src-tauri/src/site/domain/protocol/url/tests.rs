use super::*;
use crate::site::domain::protocol::codes::SafCode;

#[test]
fn the_launch_invocation_is_split_into_verb_and_parameters() {
    let url =
        AfirmaUrl::parse("afirma://websocket?ports=49152,50001,60123&v=4&jvc=3&idsession=abc123")
            .expect("la invocacion de arranque publicada deberia parsearse");

    assert_eq!(url.verb(), "websocket");
    assert_eq!(url.parameter("ports"), Some("49152,50001,60123"));
    assert_eq!(url.parameter("v"), Some("4"));
    assert_eq!(url.parameter("jvc"), Some("3"));
    assert_eq!(url.parameter("idsession"), Some("abc123"));
    assert_eq!(url.parameter("mcv"), None);
}

#[test]
fn a_url_without_query_is_still_a_verb() {
    let url = AfirmaUrl::parse("afirma://websocket").expect("un verbo suelto es valido");

    assert_eq!(url.verb(), "websocket");
    assert_eq!(url.parameter("ports"), None);
}

#[test]
fn the_scheme_is_compared_ignoring_case() {
    let url = AfirmaUrl::parse("AFIRMA://sign?op=sign").expect("el esquema no lleva mayusculas");

    assert_eq!(url.verb(), "sign");
}

#[test]
fn a_broken_url_is_still_recognised_as_coming_through_the_scheme() {
    assert!(AfirmaUrl::is_a_protocol_url(
        "AFIRMA://websocket?ports=51000"
    ));
    assert!(AfirmaUrl::is_a_protocol_url("afirma://"));
    assert!(
        AfirmaUrl::parse("afirma://").is_err(),
        "y sigue siendo rota: reconocer el esquema no es leerla"
    );
    assert!(!AfirmaUrl::is_a_protocol_url("/casa/documento.pdf"));
    assert!(!AfirmaUrl::is_a_protocol_url("afirma:/websocket"));
}

#[test]
fn anything_that_is_not_the_scheme_is_refused_as_a_parameter_error() {
    for url in [
        "https://sede.example/sign",
        "afirma:/websocket",
        "",
        "afirma://",
    ] {
        let refusal = AfirmaUrl::parse(url).expect_err("no es una invocacion del protocolo");
        assert_eq!(refusal.code(), SafCode::Params, "con {url}");
    }
}

#[test]
fn a_pair_without_a_key_is_dropped_like_in_the_original() {
    let url = AfirmaUrl::parse("afirma://sign?=huerfano&op=sign&suelto").expect("parsea");

    assert_eq!(url.parameter("op"), Some("sign"));
    assert_eq!(url.parameter(""), None);
    assert_eq!(url.parameter("suelto"), None);
}

#[test]
fn a_repeated_key_keeps_the_last_value() {
    let url = AfirmaUrl::parse("afirma://websocket?v=3&v=4").expect("parsea");

    assert_eq!(url.parameter("v"), Some("4"));
}

#[test]
fn the_value_is_decoded_and_the_key_is_not() {
    let url = AfirmaUrl::parse("afirma://sign?a%20b=uno%20dos&c=m%C3%A1s").expect("parsea");

    assert_eq!(url.parameter("a%20b"), Some("uno dos"));
    assert_eq!(url.parameter("a b"), None);
    assert_eq!(url.parameter("c"), Some("más"));
}

#[test]
fn a_plus_becomes_a_space_because_url_decoder_says_so() {
    let url = AfirmaUrl::parse("afirma://sign?dat=a+b").expect("parsea");

    assert_eq!(url.parameter("dat"), Some("a b"));
}

#[test]
fn a_broken_escape_stays_literal_instead_of_sinking_the_invocation() {
    let url = AfirmaUrl::parse("afirma://sign?dat=100%&op=%ZZ&x=%4").expect("parsea");

    assert_eq!(url.parameter("dat"), Some("100%"));
    assert_eq!(url.parameter("op"), Some("%ZZ"));
    assert_eq!(url.parameter("x"), Some("%4"));
}

#[test]
fn an_empty_value_is_not_an_absent_parameter() {
    let url = AfirmaUrl::parse("afirma://websocket?idsession=").expect("parsea");

    assert_eq!(url.parameter("idsession"), Some(""));
}
