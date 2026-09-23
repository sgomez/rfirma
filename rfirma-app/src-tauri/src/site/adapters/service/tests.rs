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
    Inbox::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
        reply.answer(text.to_owned());
    })
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

    assert_eq!(body_of(&response.0), ECHO_OK);
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

    assert!(body_of(&response.0).starts_with("SAF_"));
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

    assert_eq!(body_of(&response.0), "1");
}

#[tokio::test]
async fn a_fragment_that_is_not_the_last_asks_for_more_data() {
    let chunk = URL_SAFE.encode("mitad");
    let raw = format!("fragment=@1@2@{chunk}idsession={CREDENTIAL}@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert_eq!(body_of(&response.0), MORE_DATA_NEED);
}

#[tokio::test]
async fn the_last_fragment_is_answered_with_ok() {
    let chunk = URL_SAFE.encode("resto");
    let raw = format!("fragment=@2@2@{chunk}idsession={CREDENTIAL}@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert_eq!(body_of(&response.0), ECHO_OK);
}

#[tokio::test]
async fn a_fragment_with_the_wrong_credential_is_refused_with_saf_03() {
    let chunk = URL_SAFE.encode("mitad");
    let raw = format!("fragment=@1@2@{chunk}idsession=otraPaginaDelEquipo0@EOF");

    let response = respond(&raw, true, &serving(), &answering_with("x"), &no_state()).await;

    assert!(body_of(&response.0).starts_with("SAF_03"));
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

    assert_eq!(body_of(&response.0), "1");
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

    assert_eq!(body_of(&response.0), "resultado");
}

#[tokio::test]
async fn a_send_with_the_wrong_credential_is_refused_with_saf_03() {
    let response = respond(
        "send=@1@1idsession=otraPaginaDelEquipo0@EOF",
        true,
        &serving(),
        &answering_with("x"),
        &no_state(),
    )
    .await;

    assert!(body_of(&response.0).starts_with("SAF_03"));
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

    assert_eq!(body_of(&response.0), ECHO_OK);
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

    assert_eq!(body_of(&response.0), refusal.answer().on_the_wire());
}

/// Un buzón que cuenta cuántas operaciones se le entregan, para distinguir un relanzamiento de
/// una respuesta ya calculada.
fn counting_answers_with(text: &'static str, launches: &Arc<Mutex<usize>>) -> Inbox {
    let launches = Arc::clone(launches);
    Inbox::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
        *launches.lock().expect("el contador no esta envenenado") += 1;
        reply.answer(text.to_owned());
    })
}

fn a_command(operation: &str) -> String {
    let encoded = URL_SAFE.encode(operation);
    format!("cmd={encoded}idsession={CREDENTIAL}@EOF")
}

#[tokio::test]
async fn a_repeated_command_answers_the_number_of_parts_without_relaunching_the_operation() {
    let state = no_state();
    let launches = Arc::new(Mutex::new(0));
    let inbox = counting_answers_with("resultado", &launches);
    let raw = a_command("afirma://selectcert?op=selectcert");

    let first = respond(&raw, true, &serving(), &inbox, &state).await;
    let second = respond(&raw, true, &serving(), &inbox, &state).await;

    assert_eq!(body_of(&first.0), "1");
    assert_eq!(body_of(&second.0), "1");
    assert_eq!(*launches.lock().expect("el contador no esta envenenado"), 1);
}

#[tokio::test]
async fn a_command_with_the_wrong_credential_is_refused_with_saf_03() {
    let encoded = URL_SAFE.encode("afirma://selectcert?op=selectcert");
    let response = respond(
        &format!("cmd={encoded}idsession=0000000000000000000O@EOF"),
        true,
        &serving(),
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert_eq!(
        body_of(&response.0),
        WireAnswer::refused_because_of(SafCode::Params, Parameter::IdSession).on_the_wire()
    );
}

#[tokio::test]
async fn an_echo_that_resets_discards_the_response_already_computed() {
    let state = no_state();
    let launches = Arc::new(Mutex::new(0));
    let inbox = counting_answers_with("resultado", &launches);
    let raw = a_command("afirma://selectcert?op=selectcert");
    respond(&raw, true, &serving(), &inbox, &state).await;

    respond(
        &format!("echo=-idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &inbox,
        &state,
    )
    .await;
    let after_the_reset = respond(&raw, true, &serving(), &inbox, &state).await;

    assert_eq!(body_of(&after_the_reset.0), "1");
    assert_eq!(*launches.lock().expect("el contador no esta envenenado"), 2);
}

#[tokio::test]
async fn the_firm_response_carries_an_acknowledgement_fulfilled_once_the_write_is_confirmed() {
    let state = no_state();
    let delivered: Arc<Mutex<Vec<Acknowledgement>>> = Arc::new(Mutex::new(Vec::new()));
    let keeping = Arc::clone(&delivered);
    let inbox = Inbox::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
        keeping
            .lock()
            .expect("el candado")
            .push(reply.answer("resultado".to_owned()));
    });
    let raw = a_command("afirma://selectcert?op=selectcert");

    let (response, acknowledged) = respond(&raw, true, &serving(), &inbox, &state).await;

    assert_eq!(body_of(&response), "1");
    let acknowledged = acknowledged.expect("la respuesta a firm= trae el acuse de la entrega");
    assert!(
        !delivered.lock().expect("el candado")[0].wait(std::time::Duration::from_millis(0)),
        "el acuse no deberia cumplirse antes de que el escritor del socket lo confirme"
    );

    acknowledged.fulfil();

    assert!(
        delivered.lock().expect("el candado")[0].wait(std::time::Duration::from_millis(0)),
        "el acuse deberia cumplirse en cuanto quien escribe en el socket lo confirma"
    );
}

const A_SAVE: &str = "afirma://save?op=save&filename=reto.bin&dat=cmV0bw";

#[tokio::test]
async fn a_save_command_answers_save_ok_and_not_the_number_of_parts() {
    let response = respond(
        &a_command(A_SAVE),
        true,
        &serving(),
        &answering_with("SAVE_OK"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response.0), "SAVE_OK");
}

#[tokio::test]
async fn a_save_sent_in_fragments_answers_save_ok_to_firm() {
    let state = no_state();
    let first = URL_SAFE.encode("afirma://save?op=save&");
    let second = URL_SAFE.encode("filename=reto.bin&dat=cmV0bw");
    for fragment in [
        format!("fragment=@1@2@{first}idsession={CREDENTIAL}@EOF"),
        format!("fragment=@2@2@{second}idsession={CREDENTIAL}@EOF"),
    ] {
        respond(
            &fragment,
            true,
            &serving(),
            &answering_with("SAVE_OK"),
            &state,
        )
        .await;
    }

    let response = respond(
        &format!("firm=idsession={CREDENTIAL}@EOF"),
        true,
        &serving(),
        &answering_with("SAVE_OK"),
        &state,
    )
    .await;

    assert_eq!(body_of(&response.0), "SAVE_OK");
}

#[tokio::test]
async fn a_cancelled_save_answers_cancel() {
    let response = respond(
        &a_command(A_SAVE),
        true,
        &serving(),
        &answering_with("CANCEL"),
        &no_state(),
    )
    .await;

    assert_eq!(body_of(&response.0), "CANCEL");
}

#[tokio::test]
async fn a_save_leaves_no_parts_for_the_next_operation() {
    let state = no_state();
    let launches = Arc::new(Mutex::new(0));
    let inbox = counting_answers_with("SAVE_OK", &launches);

    respond(&a_command(A_SAVE), true, &serving(), &inbox, &state).await;
    let second = respond(&a_command(A_SAVE), true, &serving(), &inbox, &state).await;

    assert_eq!(body_of(&second.0), "SAVE_OK");
    assert_eq!(*launches.lock().expect("el contador no esta envenenado"), 2);
}

async fn answered(raw: &str, inbox: &Inbox, state: &Arc<Mutex<ServiceState>>) -> String {
    body_of(&respond(raw, true, &serving(), inbox, state).await.0)
}

#[tokio::test]
async fn an_echo_from_a_foreign_session_is_refused_with_saf_03() {
    let body = answered(
        "echo=-idsession=OtraSesionAjena00000@EOF",
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert!(body.starts_with("SAF_03"), "{body}");
}

#[tokio::test]
async fn an_unknown_order_is_refused_with_saf_03() {
    let body = answered(
        &format!("nada=idsession={CREDENTIAL}@EOF"),
        &answering_with("no se llama"),
        &no_state(),
    )
    .await;

    assert!(body.starts_with("SAF_03"), "{body}");
}

#[tokio::test]
async fn a_command_that_is_not_an_operation_is_refused_with_saf_11() {
    let launches = Arc::new(Mutex::new(0));
    let inbox = counting_answers_with("no se llama", &launches);

    for uri in [
        "afirma://service?ports=54351&v=3",
        "https://sede.example/tramite",
    ] {
        let body = answered(&a_command(uri), &inbox, &no_state()).await;

        assert!(body.starts_with("SAF_11"), "{uri}: {body}");
    }
    assert_eq!(*launches.lock().expect("el contador no esta envenenado"), 0);
}

#[tokio::test]
async fn a_send_beyond_the_announced_parts_is_refused_with_saf_11() {
    let state = no_state();
    let inbox = answering_with("resultado");
    answered(
        &a_command("afirma://selectcert?op=selectcert"),
        &inbox,
        &state,
    )
    .await;

    let beyond_the_total = answered(
        &format!("send=@3@1idsession={CREDENTIAL}@EOF"),
        &inbox,
        &state,
    )
    .await;
    let beyond_what_was_computed = answered(
        &format!("send=@2@2idsession={CREDENTIAL}@EOF"),
        &inbox,
        &state,
    )
    .await;

    assert!(beyond_the_total.starts_with("SAF_11"), "{beyond_the_total}");
    assert!(
        beyond_what_was_computed.starts_with("SAF_11"),
        "{beyond_what_was_computed}"
    );
}

#[tokio::test]
async fn a_save_that_ends_in_a_refusal_is_answered_with_saf_11() {
    let refusal = WireAnswer::refused(SafCode::Params).on_the_wire();
    let inbox = Inbox::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
        reply.answer(refusal.clone());
    });

    let body = answered(
        &a_command("afirma://save?op=save&filename=rfirma.txt&exts=txt"),
        &inbox,
        &no_state(),
    )
    .await;

    assert!(body.starts_with("SAF_11"), "{body}");
}

#[tokio::test]
async fn the_fragments_of_the_script_are_reassembled_and_run_by_firm() {
    let state = no_state();
    let inbox = answering_with("SAF_06: Formato de firma no soportado");
    let operation = "afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&dat=SG9sYQ";
    let (first, last) = operation.split_at(operation.len().div_ceil(2));
    let fragment = |part: usize, chunk: &str| {
        format!(
            "fragment=@{part}@2@{}idsession={CREDENTIAL}@EOF",
            URL_SAFE.encode(chunk)
        )
    };
    answered(&format!("echo=-idsession={CREDENTIAL}@EOF"), &inbox, &state).await;

    let early = answered(&fragment(1, first), &inbox, &state).await;
    let closing = answered(&fragment(2, last), &inbox, &state).await;
    let fired = answered(&format!("firm=idsession={CREDENTIAL}@EOF"), &inbox, &state).await;
    let sent = answered(
        &format!("send=@1@1idsession={CREDENTIAL}@EOF"),
        &inbox,
        &state,
    )
    .await;

    assert_eq!(
        [
            early.as_str(),
            closing.as_str(),
            fired.as_str(),
            sent.as_str()
        ],
        [
            MORE_DATA_NEED,
            ECHO_OK,
            "1",
            "SAF_06: Formato de firma no soportado"
        ]
    );
}

#[test]
fn only_a_recognised_order_of_the_negotiated_session_holds_the_idle_clock() {
    let echo = format!("echo=-idsession={CREDENTIAL}@EOF");

    assert!(a_valid_order(&echo, true, &serving()));
    assert!(!a_valid_order(&echo, false, &serving()));
    assert!(!a_valid_order(
        "echo=-idsession=OtraSesionAjena00000@EOF",
        true,
        &serving()
    ));
    assert!(!a_valid_order(
        &format!("nada=idsession={CREDENTIAL}@EOF"),
        true,
        &serving()
    ));
}

#[tokio::test(start_paused = true)]
async fn an_idle_channel_stops_listening_and_says_so() {
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::site::adapters::tls::LocalServerCertificate;
    use crate::site::domain::local_ca::LocalCa;

    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado del servidor local");
    let acceptor = Arc::new(acceptor_for(&certificate).expect("el aceptador TLS"));
    let listener = LoopbackListeners::from(
        std::net::TcpListener::bind("127.0.0.1:0").expect("un puerto libre del bucle local"),
    )
    .into_async()
    .expect("el escuchador en tokio");
    let idled = Arc::new(AtomicBool::new(false));
    let inbox = answering_with("no se llama").when_the_channel_idles({
        let idled = Arc::clone(&idled);
        move || idled.store(true, Ordering::SeqCst)
    });
    let (_stop, stopped) = oneshot::channel();

    let accepting = tokio::spawn(accept_until_stopped(
        listener,
        acceptor,
        serving(),
        inbox,
        no_state(),
        IdleClock::started(SOCKET_TIMEOUT),
        stopped,
    ));

    tokio::time::timeout(
        SOCKET_TIMEOUT + std::time::Duration::from_secs(1),
        accepting,
    )
    .await
    .expect("el canal deberia dejar de escuchar al vencer el reloj")
    .expect("el bucle de aceptacion no deberia fallar");
    assert!(idled.load(Ordering::SeqCst));
}
