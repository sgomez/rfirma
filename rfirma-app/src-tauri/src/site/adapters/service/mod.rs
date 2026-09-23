//! Transporte TLS crudo del protocolo `service`, sin WebSocket (ADR-0005, ADR-0017).
//!
//! Una conexión, una petición: el cliente publicado abre un socket TLS por cada `POST`, así
//! que el reensamblado de fragmentos y las partes de una respuesta larga viven en el estado
//! compartido de esta instancia de transporte (`ServiceState`), no en la conexión ni en el
//! proceso — el equivalente por instancia a los campos estáticos del original.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio_native_tls::TlsAcceptor;

mod idle;

use idle::{IdleClock, SOCKET_TIMEOUT};

use crate::lock;
use crate::site::adapters::channel::bind::LoopbackAcceptor;
use crate::site::adapters::channel::conversation::{answer, Answer, ECHO_OK};
use crate::site::adapters::channel::server::acceptor_for;
use crate::site::adapters::channel::{bind_first_free, LoopbackListeners};
use crate::site::adapters::codec::SAVE_OK;
use crate::site::adapters::tls::{LocalCaStore, LocalServerCertificate};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown, Situation,
};
use crate::site::domain::protocol::{
    credential_matches, http_response, read_request, request_credential, split_response, AfirmaUrl,
    FragmentBuffer, FramedRequest, Parameter, SafCode, WireAnswer, CANCELLED, MORE_DATA_NEED, SAVE,
};

use crate::site::application::errand::{
    Acknowledged, Acknowledgement, Inbox, ReplyHandle, Transport,
};

/// Transporte de producción del protocolo `service`: TLS crudo sobre el *loopback*.
pub struct RawTlsService {
    store: LocalCaStore,
    inbox: Inbox,
}

impl RawTlsService {
    /// Crea un transporte que emite su certificado con la CA local dada.
    pub fn new(store: LocalCaStore, inbox: Inbox) -> Self {
        Self { store, inbox }
    }
}

impl Transport for RawTlsService {
    fn open(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        let unusable = |detail: String| ChannelError::new(Situation::MaterialNotUsable, detail);
        let ca = self
            .store
            .read()
            .map_err(|error| unusable(error.to_string()))?
            .ok_or_else(|| {
                unusable(
                    "no hay CA local con la que firmar el certificado del servidor local"
                        .to_owned(),
                )
            })?;
        let certificate =
            LocalServerCertificate::issued_by(&ca).map_err(|error| unusable(error.to_string()))?;

        open(location, &certificate, duty, self.inbox.clone())
    }
}

/// El reensamblado de fragmentos y las partes de la última respuesta calculada, compartidos
/// entre las conexiones de una misma instancia de transporte (los campos estáticos del
/// original, por instancia y no por proceso).
#[derive(Default)]
struct ServiceState {
    fragments: FragmentBuffer,
    parts: Vec<String>,
}

impl ServiceState {
    fn reset(&mut self) {
        self.fragments.reset();
        self.parts.clear();
    }
}

/// Enlaza la ubicación indicada y arranca el servidor del transporte `service`.
fn open(
    location: &ChannelLocation,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    inbox: Inbox,
) -> Result<OpenChannel, ChannelError> {
    let listener = bind_first_free(location)?;
    tauri::async_runtime::block_on(serve(listener, certificate, duty, inbox))
}

async fn serve(
    listener: LoopbackListeners,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    inbox: Inbox,
) -> Result<OpenChannel, ChannelError> {
    let not_listening =
        |error: std::io::Error| ChannelError::new(Situation::NotListening, error.to_string());
    let port = listener.port().map_err(not_listening)?;
    let acceptor = Arc::new(acceptor_for(certificate)?);
    let listener = listener.into_async().map_err(not_listening)?;

    let (stop, stopped) = oneshot::channel();
    let state = Arc::new(Mutex::new(ServiceState::default()));
    let clock = IdleClock::started(SOCKET_TIMEOUT);
    tokio::spawn(accept_until_stopped(
        listener, acceptor, duty, inbox, state, clock, stopped,
    ));

    Ok(OpenChannel::new(
        port,
        Shutdown::of(move || {
            let _ = stop.send(());
        }),
    ))
}

async fn accept_until_stopped(
    listener: LoopbackAcceptor,
    acceptor: Arc<TlsAcceptor>,
    duty: ChannelDuty,
    inbox: Inbox,
    state: Arc<Mutex<ServiceState>>,
    clock: IdleClock,
    stopped: oneshot::Receiver<()>,
) {
    tokio::pin!(stopped);

    loop {
        tokio::select! {
            _ = &mut stopped => break,
            () = clock.expired() => {
                inbox.channel_went_idle();
                break;
            }
            accepted = listener.accept() => {
                let Ok((stream, peer)) = accepted else { continue };
                let acceptor = Arc::clone(&acceptor);
                let duty = duty.clone();
                let inbox = inbox.clone();
                let state = Arc::clone(&state);
                let clock = clock.clone();
                tokio::spawn(async move {
                    attend(stream, peer, &acceptor, &duty, &inbox, &state, &clock).await;
                });
            }
        }
    }
}

async fn attend(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    acceptor: &TlsAcceptor,
    duty: &ChannelDuty,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
    clock: &IdleClock,
) {
    let Ok(mut encrypted) = acceptor.accept(stream).await else {
        return;
    };
    let from_loopback = peer.ip().is_loopback();

    let Some(raw) = the_framed_request(&mut encrypted).await else {
        return;
    };

    let _in_flight = a_valid_order(&raw, from_loopback, duty).then(|| clock.order_arrived());
    let (response, acknowledged) = respond(&raw, from_loopback, duty, inbox, state).await;
    let delivered = encrypted.write_all(&response).await.is_ok();
    let _ = encrypted.shutdown().await;
    if delivered {
        if let Some(acknowledged) = acknowledged {
            acknowledged.fulfil();
        }
    }
}

/// Una orden que para el reloj de inactividad: las rechazadas antes de reconocerla no lo reinician.
fn a_valid_order(raw: &str, from_loopback: bool, duty: &ChannelDuty) -> bool {
    let ChannelDuty::Serve(credential) = duty else {
        return false;
    };
    from_loopback
        && credential_matches(credential, request_credential(raw).as_deref())
        && read_request(raw).is_ok()
}

const THE_EOF_MARK: &[u8] = b"@EOF";

/// Lee del socket hasta encontrar `@EOF`, o `None` si el par cierra antes de completarla.
async fn the_framed_request(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];

    loop {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(read) => {
                let unscanned = buffer.len().saturating_sub(THE_EOF_MARK.len() - 1);
                buffer.extend_from_slice(&chunk[..read]);
                if buffer[unscanned..]
                    .windows(THE_EOF_MARK.len())
                    .any(|window| window == THE_EOF_MARK)
                {
                    return Some(String::from_utf8_lossy(&buffer).into_owned());
                }
            }
        }
    }
}

async fn respond(
    raw: &str,
    from_loopback: bool,
    duty: &ChannelDuty,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
) -> (Vec<u8>, Option<Acknowledged>) {
    if !from_loopback {
        return (refused(SafCode::ExternalRequestToSocket), None);
    }

    let credential = match duty {
        ChannelDuty::Refuse(answer) => return (http_response(&answer.on_the_wire()), None),
        ChannelDuty::Serve(credential) => credential,
    };
    if !credential_matches(credential, request_credential(raw).as_deref()) {
        return (the_invalid_session_response(), None);
    }

    let request = match read_request(raw) {
        Ok(request) => request,
        Err(refused_order) => return (refused(refused_order.code()), None),
    };

    match request {
        FramedRequest::Echo { message, resets } => {
            if resets {
                lock(state).reset();
            }
            handle_operation(&message, duty, from_loopback, inbox, state).await
        }
        FramedRequest::Command { message } => {
            handle_operation(&message, duty, from_loopback, inbox, state).await
        }
        FramedRequest::Fragment {
            part, total, chunk, ..
        } => {
            lock(state).fragments.insert(part, chunk);
            let answer = if part == total {
                ECHO_OK
            } else {
                MORE_DATA_NEED
            };
            (http_response(answer), None)
        }
        FramedRequest::Firm { .. } => {
            inbox.arrived();
            if let Some(response) = the_response_already_computed(state) {
                return (response, None);
            }
            let combined = lock(state).fragments.combined();
            let Some(url) = combined.and_then(|message| AfirmaUrl::parse(&message).ok()) else {
                return (refused(SafCode::UnsupportedOperation), None);
            };
            launch_operation(url, inbox, state).await
        }
        FramedRequest::Send { part, .. } => match lock(state).parts.get(part - 1) {
            Some(computed) => (http_response(computed), None),
            None => (refused(SafCode::SendingResult), None),
        },
    }
}

fn refused(code: SafCode) -> Vec<u8> {
    http_response(&WireAnswer::refused(code).on_the_wire())
}

/// El número de partes de una respuesta ya calculada, para que un reintento de `cmd=` o de
/// `firm=` no relance la operación (`if (toSend.isEmpty())`, líneas 271 y 330).
fn the_response_already_computed(state: &Arc<Mutex<ServiceState>>) -> Option<Vec<u8>> {
    let parts = lock(state).parts.len();
    (parts > 0).then(|| http_response(&parts.to_string()))
}

fn the_invalid_session_response() -> Vec<u8> {
    http_response(
        &WireAnswer::refused_because_of(SafCode::Params, Parameter::IdSession).on_the_wire(),
    )
}

async fn handle_operation(
    message: &str,
    duty: &ChannelDuty,
    from_loopback: bool,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
) -> (Vec<u8>, Option<Acknowledged>) {
    match answer(duty, from_loopback, message) {
        Answer::Reply(text) => {
            inbox.arrived();
            (http_response(&text), None)
        }
        Answer::ReplyAndClose(text) | Answer::Refuse(text) => (http_response(&text), None),
        Answer::Pending(url) => {
            inbox.arrived();
            match the_response_already_computed(state) {
                Some(response) => (response, None),
                None => launch_operation(url, inbox, state).await,
            }
        }
    }
}

/// Entrega la operación al trámite y espera su resultado, ya troceado en partes (`toSend`,
/// `calculateNumberPartsResponse` en el original): la respuesta a `cmd=`/`firm=` es el número de
/// partes, que `send=` reparte luego, salvo en un guardado, que contesta su confirmación sin
/// trocear (líneas 290-305 y 333-346). El acuse acompaña esta respuesta porque es la única que
/// nace de la entrega al trámite: las de `send=` solo reparten lo ya calculado.
async fn launch_operation(
    url: AfirmaUrl,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
) -> (Vec<u8>, Option<Acknowledged>) {
    let answers_without_parts = url.verb() == SAVE;
    let (sender, receiver) = oneshot::channel();
    let (acknowledged, acknowledgement) = Acknowledgement::pair();
    inbox.deliver(
        url,
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
            acknowledgement
        }),
    );
    let Ok(result) = receiver.await else {
        return (refused(SafCode::UnsupportedOperation), None);
    };
    if answers_without_parts {
        return (the_save_confirmation(&result), Some(acknowledged));
    }
    let mut state = lock(state);
    state.parts = split_response(&result);
    (
        http_response(&state.parts.len().to_string()),
        Some(acknowledged),
    )
}

/// `SAVE_OK` o `CANCEL` tal cual; cualquier otro desenlace del guardado, `SAF_11` (líneas 341-345).
fn the_save_confirmation(result: &str) -> Vec<u8> {
    if result == SAVE_OK || result == CANCELLED {
        http_response(result)
    } else {
        refused(SafCode::SendingResult)
    }
}

#[cfg(test)]
mod tests;
