use std::sync::{Arc, Mutex};

use super::*;
use crate::site::application::tests::InMemoryServlets;
use crate::site::domain::protocol::{encrypt, AfirmaUrl, CipherKey, NegotiatedCredential, SafCode};
use crate::site::domain::relay_error::Situation as RelaySituation;

const KEY: &str = "12345678";
const RETRIEVE_SERVLET: &str = "https://relay.example/retrieve";
const STORE_SERVLET: &str = "https://relay.example/store";

fn a_key() -> CipherKey {
    CipherKey::from_url_parameter(KEY)
        .expect("longitud correcta")
        .expect("un valor no vacio siempre produce una clave")
}

fn an_operation(text: &str) -> AfirmaUrl {
    AfirmaUrl::parse(text).expect("la URL de operacion deberia parsear")
}

/// Registra el orden en el que se llama a `wait`, `retrieve` y `store`, delegando en un doble en memoria.
#[derive(Default)]
struct OrderedSpy {
    log: Mutex<Vec<&'static str>>,
    body: InMemoryServlets,
    rejects_store: bool,
}

impl OrderedSpy {
    fn that_rejects_the_upload() -> Self {
        Self {
            rejects_store: true,
            ..Self::default()
        }
    }

    fn log(&self) -> Vec<&'static str> {
        self.log.lock().expect("el candado").clone()
    }
}

impl Servlets for OrderedSpy {
    fn retrieve(&self, service_url: &str, id: &str) -> Result<String, RelayError> {
        self.log.lock().expect("el candado").push("get");
        self.body.retrieve(service_url, id)
    }

    fn store(&self, service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        self.log.lock().expect("el candado").push("put");
        if self.rejects_store {
            return Err(RelayError::new(
                RelaySituation::UploadRejected,
                "el servlet rechaza la subida",
            ));
        }
        self.body.store(service_url, id, data)
    }

    fn wait(&self, service_url: &str, id: &str) -> Result<(), RelayError> {
        self.log.lock().expect("el candado").push("wait");
        self.body.wait(service_url, id)
    }
}

/// Lo que el buzón y los dos avisos del transporte recibieron.
struct Spy {
    delivered: Arc<Mutex<Option<(AfirmaUrl, ReplyHandle)>>>,
    exits: Arc<Mutex<u32>>,
    failures: Arc<Mutex<Vec<Refusal>>>,
}

impl Spy {
    fn take_reply(&self) -> (AfirmaUrl, ReplyHandle) {
        self.delivered
            .lock()
            .expect("el candado")
            .take()
            .expect("la operacion deberia haberse entregado")
    }

    fn exits(&self) -> u32 {
        *self.exits.lock().expect("el candado")
    }

    fn failures(&self) -> Vec<Refusal> {
        self.failures.lock().expect("el candado").clone()
    }
}

fn a_relay(servlets: Arc<OrderedSpy>) -> (Relay, Spy) {
    let delivered = Arc::new(Mutex::new(None));
    let exits = Arc::new(Mutex::new(0u32));
    let failures = Arc::new(Mutex::new(Vec::new()));

    let inbox_delivered = Arc::clone(&delivered);
    let inbox: Inbox = Arc::new(move |url, reply| {
        *inbox_delivered.lock().expect("el candado") = Some((url, reply));
    });

    let exit_count = Arc::clone(&exits);
    let exit: Arc<dyn Fn() + Send + Sync> =
        Arc::new(move || *exit_count.lock().expect("el candado") += 1);

    let failure_log = Arc::clone(&failures);
    let on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync> =
        Arc::new(move |refusal| failure_log.lock().expect("el candado").push(refusal));

    let relay = Relay::new(
        servlets as Arc<dyn Servlets + Send + Sync>,
        inbox,
        exit,
        on_upload_failure,
    );
    (
        relay,
        Spy {
            delivered,
            exits,
            failures,
        },
    )
}

fn duty() -> ChannelDuty {
    ChannelDuty::Serve(NegotiatedCredential::Absent)
}

/// Abre el canal y dispara su entrega diferida, como haría `attend_launch` tras registrar el
/// trámite.
fn opened_and_delivered(relay: &Relay, info: &ChannelLocation) -> OpenChannel {
    let mut channel = relay.open(info, duty()).expect("abre y entrega");
    channel
        .take_delivery()
        .expect("una operacion Serve siempre trae entrega")
        .now();
    channel
}

fn a_fileid_info(
    retrieve_servlet: Option<&str>,
    key: Option<CipherKey>,
    active_wait: bool,
) -> RelayChannelInfo {
    RelayChannelInfo {
        operation: an_operation("afirma://sign?algorithm=SHA256withRSA"),
        retrieve_servlet: retrieve_servlet.map(str::to_owned),
        store_servlet: STORE_SERVLET.to_owned(),
        id: "tx-1".to_owned(),
        fileid: Some("fileid-1".to_owned()),
        key,
        active_wait,
    }
}

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
    let info = ChannelLocation::Relay(a_fileid_info(Some(RETRIEVE_SERVLET), Some(key), false));

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
        retrieve_servlet: None,
        store_servlet: STORE_SERVLET.to_owned(),
        id: "tx-2".to_owned(),
        fileid: None,
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
    let info = ChannelLocation::Relay(a_fileid_info(Some(RETRIEVE_SERVLET), Some(key), true));

    opened_and_delivered(&relay, &info);

    assert_eq!(servlets.log(), vec!["wait", "get"]);
}

#[test]
fn a_successful_upload_closes_the_process_and_reports_no_failure() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(RelayChannelInfo {
        operation: an_operation("afirma://sign?dat=algo&algorithm=SHA256withRSA"),
        retrieve_servlet: None,
        store_servlet: STORE_SERVLET.to_owned(),
        id: "tx-3".to_owned(),
        fileid: None,
        key: None,
        active_wait: false,
    });

    opened_and_delivered(&relay, &info);
    let (_operation, reply) = spy.take_reply();
    reply.answer("la-respuesta-cifrada".to_owned());

    assert_eq!(
        servlets.body.retrieve(STORE_SERVLET, "tx-3"),
        Ok("la-respuesta-cifrada".to_owned())
    );
    assert_eq!(spy.exits(), 1);
    assert!(spy.failures().is_empty());
}

#[test]
fn a_rejected_upload_notifies_without_closing_the_process() {
    let servlets = Arc::new(OrderedSpy::that_rejects_the_upload());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(RelayChannelInfo {
        operation: an_operation("afirma://sign?dat=algo&algorithm=SHA256withRSA"),
        retrieve_servlet: None,
        store_servlet: STORE_SERVLET.to_owned(),
        id: "tx-4".to_owned(),
        fileid: None,
        key: None,
        active_wait: false,
    });

    opened_and_delivered(&relay, &info);
    let (_operation, reply) = spy.take_reply();
    reply.answer("la-respuesta-cifrada".to_owned());

    assert_eq!(spy.exits(), 0);
    let failures = spy.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].code(), SafCode::SendingResult);
}

#[test]
fn an_unreachable_servlet_refuses_with_saf_16_without_delivering_anything() {
    let servlets = Arc::new(OrderedSpy {
        body: InMemoryServlets::unreachable(),
        ..OrderedSpy::default()
    });
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(Some(RETRIEVE_SERVLET), Some(a_key()), false));

    let error = relay
        .open(&info, duty())
        .expect_err("un servlet inalcanzable no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(refusal.code(), SafCode::RecoveringData);
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
    let info = ChannelLocation::Relay(a_fileid_info(Some(RETRIEVE_SERVLET), Some(key), false));

    let error = relay
        .open(&info, duty())
        .expect_err("un contenido indescifrable no abre");

    let refusal = error.refusal().expect("trae su propio rechazo clasificado");
    assert_eq!(refusal.code(), SafCode::DecryptingData);
}

#[test]
fn a_refuse_duty_uploads_the_given_answer_without_waiting_resolving_or_delivering() {
    let servlets = Arc::new(OrderedSpy::default());
    let (relay, spy) = a_relay(Arc::clone(&servlets));
    let info = ChannelLocation::Relay(a_fileid_info(Some(RETRIEVE_SERVLET), Some(a_key()), true));
    let answer = Refusal::new(SafCode::CannotOpenSocket, "ya hay un tramite vivo").answer();

    relay
        .open(&info, ChannelDuty::Refuse(answer))
        .expect("sube el rechazo");

    assert_eq!(servlets.log(), vec!["put"]);
    assert!(spy.delivered.lock().expect("el candado").is_none());
    assert_eq!(spy.exits(), 1);
}
