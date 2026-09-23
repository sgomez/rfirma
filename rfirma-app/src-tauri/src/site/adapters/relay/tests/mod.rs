use std::sync::{Arc, Mutex};

use base64::Engine as _;

use super::*;
use crate::site::application::tests::{read_operation, InMemoryServlets};
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    encrypt, AfirmaUrl, CipherKey, NegotiatedCredential, RelayRequest, SafCode, SiteOperation,
};
use crate::site::domain::relay_error::Situation as RelaySituation;

const KEY: &str = "12345678";
const RETRIEVE_SERVLET: &str = "https://relay.example/retrieve";
const STORE_SERVLET: &str = "https://relay.example/store";
const NO_WAIT: std::time::Duration = std::time::Duration::from_millis(0);

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

/// Lo que el buzón y el aviso de fallo del transporte recibieron.
struct Spy {
    delivered: Arc<Mutex<Option<(AfirmaUrl, ReplyHandle)>>>,
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

    fn failures(&self) -> Vec<Refusal> {
        self.failures.lock().expect("el candado").clone()
    }
}

fn a_relay(servlets: Arc<OrderedSpy>) -> (Relay, Spy) {
    a_relay_on(servlets, crate::site::application::tests::a_runtime())
}

fn a_relay_on(servlets: Arc<OrderedSpy>, runtime: tokio::runtime::Handle) -> (Relay, Spy) {
    let delivered = Arc::new(Mutex::new(None));
    let failures = Arc::new(Mutex::new(Vec::new()));

    let inbox_delivered = Arc::clone(&delivered);
    let inbox = Inbox::for_operations(move |url, reply| {
        *inbox_delivered.lock().expect("el candado") = Some((url, reply));
    });

    let failure_log = Arc::clone(&failures);
    let on_upload_failure: Arc<dyn Fn(Refusal) + Send + Sync> =
        Arc::new(move |refusal| failure_log.lock().expect("el candado").push(refusal));

    let relay = Relay::new(
        servlets as Arc<dyn Servlets + Send + Sync>,
        inbox,
        on_upload_failure,
        runtime,
    );
    (
        relay,
        Spy {
            delivered,
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
    retrieve_servlet: &str,
    key: Option<CipherKey>,
    active_wait: bool,
) -> RelayChannelInfo {
    RelayChannelInfo {
        operation: an_operation("afirma://sign?algorithm=SHA256withRSA"),
        request: RelayRequest::DataByFileId {
            store_servlet: STORE_SERVLET.to_owned(),
            id: "tx-1".to_owned(),
            fileid: "fileid-1".to_owned(),
            retrieve_servlet: retrieve_servlet.to_owned(),
        },
        key,
        active_wait,
    }
}

/// El arranque que solo trae `fileid`: lo recuperado es el XML de parámetros de la operación.
fn a_parameters_info(fileid: &str, key: Option<CipherKey>) -> RelayChannelInfo {
    RelayChannelInfo {
        operation: an_operation("afirma://sign?jvc=3"),
        request: RelayRequest::ParametersByFileId {
            fileid: fileid.to_owned(),
            retrieve_servlet: RETRIEVE_SERVLET.to_owned(),
        },
        key,
        active_wait: false,
    }
}

/// El XML de parámetros que sube la sede: `<op><e k="…" v="…"/>…</op>` (`autoscript.js:4392`).
fn a_parameters_xml(pairs: &[(&str, &str)]) -> Vec<u8> {
    let mut xml = String::from("<sign>");
    for (key, value) in pairs {
        xml.push_str(&format!("<e k=\"{key}\" v=\"{value}\"/>"));
    }
    xml.push_str("</sign>");
    xml.into_bytes()
}

mod active_wait;
mod document_variant;
mod parameters_variant;
