use super::*;
use crate::site::domain::protocol::codes::SafCode;

#[test]
fn the_root_element_names_the_verb_and_every_entry_becomes_a_parameter() {
    let operation = operation_of_the_parameters_xml(
        br#"<?xml version="1.0" encoding="UTF-8"?><sign><e k="op" v="sign"/><e k="format" v="PAdES"/><e k="algorithm" v="SHA256withRSA"/></sign>"#,
    )
    .expect("el XML de parametros deberia leerse");

    assert_eq!(operation.verb(), "sign");
    assert_eq!(operation.parameter("format"), Some("PAdES"));
    assert_eq!(operation.parameter("algorithm"), Some("SHA256withRSA"));
}

#[test]
fn a_root_named_like_the_operation_parameter_means_the_default_verb() {
    let operation = operation_of_the_parameters_xml(br#"<op><e k="id" v="tx-1"/></op>"#)
        .expect("el XML de parametros deberia leerse");

    assert_eq!(operation.verb(), "sign");
    assert_eq!(operation.parameter("id"), Some("tx-1"));
}

#[test]
fn the_values_arrive_url_encoded_and_come_out_decoded() {
    let operation = operation_of_the_parameters_xml(
        br#"<selectcert><e k="stservlet" v="https%3A%2F%2Fsede.example%2Fstore"/></selectcert>"#,
    )
    .expect("el XML de parametros deberia leerse");

    assert_eq!(
        operation.parameter("stservlet"),
        Some("https://sede.example/store")
    );
}

#[test]
fn the_xml_entities_of_an_attribute_come_out_resolved() {
    let operation =
        operation_of_the_parameters_xml(br#"<sign><e k="properties" v="a&amp;b"/></sign>"#)
            .expect("el XML de parametros deberia leerse");

    assert_eq!(operation.parameter("properties"), Some("a&b"));
}

#[test]
fn a_child_that_is_not_an_entry_is_refused_as_a_parameters_problem() {
    let refusal = operation_of_the_parameters_xml(br#"<sign><otra k="op" v="sign"/></sign>"#)
        .expect_err("el original solo admite entradas 'e'");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn an_entry_without_its_two_attributes_is_refused_as_a_parameters_problem() {
    let refusal = operation_of_the_parameters_xml(br#"<sign><e k="op"/></sign>"#)
        .expect_err("una entrada sin 'v' no vale");

    assert_eq!(refusal.code(), SafCode::Params);
}

#[test]
fn something_that_is_not_xml_is_refused_as_a_parameters_problem() {
    let refusal = operation_of_the_parameters_xml(b"esto no es un XML de parametros")
        .expect_err("no hay operacion que leer");

    assert_eq!(refusal.code(), SafCode::Params);
}
