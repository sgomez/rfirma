//! Pruebas del arranque por servidor intermedio: su destino, sus servlets y su clave.

use super::super::*;
use crate::site::domain::channel::ChannelLocation;

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
fn a_refusal_location_of_a_relay_launch_whose_stservlet_is_local_is_none() {
    let url = AfirmaUrl::parse(
        "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=http://127.0.0.1:8080/store\
         &id=tx2",
    )
    .expect("es una URL de servidor intermedio");

    assert_eq!(
        location_for_a_refusal(&url),
        None,
        "un stservlet que se rechaza por local no recibe el rechazo"
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
fn a_relay_selectcert_without_dat_uploads_to_the_url_destination() {
    let request = LaunchRequest::parse(
        "afirma://selectcert?op=selectcert&stservlet=https://relay.example/store&id=tx5",
    )
    .expect("la seleccion de certificado no lleva datos que operar");

    let ChannelLocation::Relay(info) = request.location() else {
        panic!("una operacion con servlet negocia canal de servidor intermedio");
    };
    assert_eq!(
        info.request,
        RelayRequest::Inline {
            store_servlet: "https://relay.example/store".to_owned(),
            id: "tx5".to_owned(),
        }
    );
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
fn a_cipher_key_of_eight_characters_that_are_not_eight_bytes_is_refused_naming_the_key() {
    let refusal = LaunchRequest::parse(
        "afirma://sign?dat=ZmlybWFkbw&stservlet=https://relay.example/store&id=tx1&\
         key=%C3%B1%C3%B1%C3%B1%C3%B1%C3%B1%C3%B1%C3%B1%C3%B1",
    )
    .expect_err("ocho eñes no son una clave DES");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::CipherKey));
}
