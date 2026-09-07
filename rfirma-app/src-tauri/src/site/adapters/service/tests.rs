use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;

use super::*;
use crate::site::domain::protocol::{AfirmaUrl, ChannelCredential, NegotiatedCredential};

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

fn serving() -> ChannelDuty {
    ChannelDuty::Serve(NegotiatedCredential::Required(
        ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
    ))
}

fn serving_without_credential() -> ChannelDuty {
    ChannelDuty::Serve(NegotiatedCredential::Absent)
}

fn no_state() -> Arc<Mutex<ServiceState>> {
    Arc::new(Mutex::new(ServiceState::default()))
}

/// Un buzón que contesta cada operación con el mismo texto, sin atender de verdad.
fn answering_with(text: &'static str) -> Inbox {
    Arc::new(move |_url: AfirmaUrl, reply: ReplyHandle| reply.answer(text.to_owned()))
}

fn body_of(response: &[u8]) -> String {
    let text = String::from_utf8(response.to_vec()).expect("la respuesta es utf-8");
    let body = text.split("\n\n").nth(1).expect("la respuesta trae cuerpo");
    let decoded = URL_SAFE.decode(body).expect("el cuerpo es base64 valido");
    String::from_utf8(decoded).expect("el cuerpo decodificado es utf-8")
}

#[tokio::test]
async fn an_echo_is_answered_with_ok() {
    let response = respond(
        &format!("echo=-idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response), ECHO_OK);
}

#[tokio::test]
async fn a_request_from_outside_the_loopback_is_refused() {
    let response = respond(
        &format!("echo=-idsession={CREDENTIAL}@EOF"),
        false,
        &serving(),
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert!(body_of(&response).starts_with("SAF_"));
}

#[tokio::test]
async fn a_command_is_delivered_and_the_response_is_the_number_of_parts() {
    let command = URL_SAFE.encode("afirma://selectcert?op=selectcert");
    let raw = format!("cmd={command}idsession={CREDENTIAL}@EOF");

    let response = respond(
        &raw,
        true,
        &serving(),
        &answering_with("una_firma_corta"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response), "1");
}

#[tokio::test]
async fn a_fragment_that_is_not_the_last_asks_for_more_data() {
    let chunk = URL_SAFE.encode("mitad");
    let raw = format!("fragment=@1@2@{chunk}idsession={CREDENTIAL}@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert_eq!(body_of(&response), MORE_DATA_NEED);
}

#[tokio::test]
async fn the_last_fragment_is_answered_with_ok() {
    let chunk = URL_SAFE.encode("resto");
    let raw = format!("fragment=@2@2@{chunk}idsession={CREDENTIAL}@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert_eq!(body_of(&response), ECHO_OK);
}

#[tokio::test]
async fn a_fragment_with_the_wrong_credential_is_refused() {
    let chunk = URL_SAFE.encode("mitad");
    let raw = format!("fragment=@1@2@{chunk}idsession=otraPaginaDelEquipo0@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert!(body_of(&response).starts_with("SAF_46"));
}

#[tokio::test]
async fn a_firm_combines_the_fragments_and_answers_with_the_number_of_parts() {
    let state = no_state();
    let first = URL_SAFE.encode("afirma://selectcert?");
    let second = URL_SAFE.encode("op=selectcert");
    respond(
        &format!("fragment=@1@2@{first}idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("firmado"),
        &state,
    )
    .await;
    respond(
        &format!("fragment=@2@2@{second}idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("firmado"),
        &state,
    )
    .await;

    let response = respond(
        &format!("firm=idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("firmado"),
        &state,
    )
    .await;

    assert_eq!(body_of(&response), "1");
}

#[tokio::test]
async fn a_send_returns_the_part_that_firm_already_computed() {
    let state = no_state();
    let chunk = URL_SAFE.encode("afirma://selectcert?op=selectcert");
    respond(
        &format!("fragment=@1@1@{chunk}idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("resultado"),
        &state,
    )
    .await;
    respond(
        &format!("firm=idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("resultado"),
        &state,
    )
    .await;

    let response = respond(
        &format!("send=@1@1idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("resultado"),
        &state,
    )
    .await;

    assert_eq!(body_of(&response), "resultado");
}

#[tokio::test]
async fn a_send_with_the_wrong_credential_is_refused() {
    let response = respond(
        "send=@1@1idsession=otraPaginaDelEquipo0@EOF",
        true,
        &serving(),
        &answering_with("x"),
        &no_state(),
    )
    .await;

    assert!(body_of(&response).starts_with("SAF_46"));
}

#[tokio::test]
async fn without_a_negotiated_credential_a_fragment_without_one_is_accepted() {
    let chunk = URL_SAFE.encode("afirma://selectcert?op=selectcert");
    let raw = format!("fragment=@1@1@{chunk}@EOF");

    let response = respond(
        &raw,
        true,
        &serving_without_credential(),
        &answering_with("x"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response), ECHO_OK);
}

#[tokio::test]
async fn a_refusing_duty_answers_the_same_refusal_regardless_of_the_command() {
    use crate::site::domain::protocol::{Refusal, RefusalSituation};

    let refusal =
        Refusal::about(Parameter::IdSession, "detalle").because(RefusalSituation::ErrandInFlight);
    let duty = ChannelDuty::Refuse(refusal.answer());

    let response = respond(
        &format!("echo=-idsession={CREDENTIAL}@EOF"),
        true,
        &duty,
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response), refusal.answer().on_the_wire());
}
