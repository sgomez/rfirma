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

use crate::lock;
use crate::site::adapters::channel::bind_first_free;
use crate::site::adapters::channel::conversation::{answer, Answer, ECHO_OK};
use crate::site::adapters::channel::server::acceptor_for;
use crate::site::adapters::tls::{LocalCaStore, LocalServerCertificate};
use crate::site::domain::channel::{
    ChannelDuty, ChannelError, ChannelLocation, OpenChannel, Shutdown, Situation,
};
use crate::site::domain::protocol::{
    credential_matches, http_response, read_request, split_response, AfirmaUrl, FragmentBuffer,
    FramedRequest, NotOfTheFraming, Parameter, SafCode, WireAnswer, MORE_DATA_NEED,
};

use crate::site::application::errand::{Inbox, ReplyHandle, Transport};

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

        open(location, &certificate, duty, Arc::clone(&self.inbox))
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
    listener: std::net::TcpListener,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    inbox: Inbox,
) -> Result<OpenChannel, ChannelError> {
    let port = listener
        .local_addr()
        .map_err(|error| ChannelError::new(Situation::NotListening, error.to_string()))?
        .port();
    let acceptor = Arc::new(acceptor_for(certificate)?);

    listener
        .set_nonblocking(true)
        .map_err(|error| ChannelError::new(Situation::NotListening, error.to_string()))?;
    let listener = tokio::net::TcpListener::from_std(listener)
        .map_err(|error| ChannelError::new(Situation::NotListening, error.to_string()))?;

    let (stop, stopped) = oneshot::channel();
    let state = Arc::new(Mutex::new(ServiceState::default()));
    tokio::spawn(accept_until_stopped(
        listener, acceptor, duty, inbox, state, stopped,
    ));

    Ok(OpenChannel::new(
        port,
        Shutdown::of(move || {
            let _ = stop.send(());
        }),
    ))
}

async fn accept_until_stopped(
    listener: tokio::net::TcpListener,
    acceptor: Arc<TlsAcceptor>,
    duty: ChannelDuty,
    inbox: Inbox,
    state: Arc<Mutex<ServiceState>>,
    stopped: oneshot::Receiver<()>,
) {
    tokio::pin!(stopped);

    loop {
        tokio::select! {
            _ = &mut stopped => break,
            accepted = listener.accept() => {
                let Ok((stream, peer)) = accepted else { continue };
                let acceptor = Arc::clone(&acceptor);
                let duty = duty.clone();
                let inbox = Arc::clone(&inbox);
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    attend(stream, peer, &acceptor, &duty, &inbox, &state).await;
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
) {
    let Ok(mut encrypted) = acceptor.accept(stream).await else {
        return;
    };
    let from_loopback = peer.ip().is_loopback();

    let Some(raw) = the_framed_request(&mut encrypted).await else {
        return;
    };

    let response = respond(&raw, from_loopback, duty, inbox, state).await;
    let _ = encrypted.write_all(&response).await;
    let _ = encrypted.shutdown().await;
}

/// Lee del socket hasta encontrar `@EOF`, o `None` si el par cierra antes de completarla.
async fn the_framed_request(stream: &mut (impl tokio::io::AsyncRead + Unpin)) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];

    loop {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"@EOF") {
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
) -> Vec<u8> {
    if !from_loopback {
        return http_response(&WireAnswer::refused(SafCode::ExternalRequestToSocket).on_the_wire());
    }

    let credential = match duty {
        ChannelDuty::Refuse(answer) => return http_response(&answer.on_the_wire()),
        ChannelDuty::Serve(credential) => credential,
    };

    let request = match read_request(raw) {
        Ok(request) => request,
        Err(NotOfTheFraming) => {
            return http_response(&WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire())
        }
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
            part,
            total,
            chunk,
            credential: candidate,
        } => {
            if !credential_matches(credential, candidate.as_deref()) {
                return the_invalid_session_response();
            }
            let mut state = lock(state);
            state.fragments.insert(part, chunk);
            http_response(if part == total {
                ECHO_OK
            } else {
                MORE_DATA_NEED
            })
        }
        FramedRequest::Firm {
            credential: candidate,
        } => {
            if !credential_matches(credential, candidate.as_deref()) {
                return the_invalid_session_response();
            }
            let already_computed = {
                let state = lock(state);
                (!state.parts.is_empty()).then(|| state.parts.len())
            };
            if let Some(parts) = already_computed {
                return http_response(&parts.to_string());
            }
            let combined = lock(state).fragments.combined();
            let Some(url) = combined.and_then(|message| AfirmaUrl::parse(&message).ok()) else {
                return http_response(
                    &WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire(),
                );
            };
            launch_operation(url, inbox, state).await
        }
        FramedRequest::Send {
            part,
            total,
            credential: candidate,
        } => {
            if !credential_matches(credential, candidate.as_deref()) {
                return the_invalid_session_response();
            }
            let state = lock(state);
            if part < 1 || part > total || part > state.parts.len() {
                return http_response(
                    &WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire(),
                );
            }
            http_response(&state.parts[part - 1])
        }
    }
}

fn the_invalid_session_response() -> Vec<u8> {
    http_response(
        &WireAnswer::refused_because_of(SafCode::InvalidSessionId, Parameter::IdSession)
            .on_the_wire(),
    )
}

async fn handle_operation(
    message: &str,
    duty: &ChannelDuty,
    from_loopback: bool,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
) -> Vec<u8> {
    match answer(duty, from_loopback, message) {
        Answer::Reply(text) | Answer::ReplyAndClose(text) => http_response(&text),
        Answer::Pending(url) => launch_operation(url, inbox, state).await,
    }
}

/// Entrega la operación al trámite y espera su resultado, ya troceado en partes (`toSend`,
/// `calculateNumberPartsResponse` en el original): la respuesta a `cmd=`/`firm=` es el número de
/// partes, que `send=` reparte luego.
async fn launch_operation(
    url: AfirmaUrl,
    inbox: &Inbox,
    state: &Arc<Mutex<ServiceState>>,
) -> Vec<u8> {
    let (sender, receiver) = oneshot::channel();
    inbox(
        url,
        ReplyHandle::of(move |text| {
            let _ = sender.send(text);
        }),
    );
    let Ok(result) = receiver.await else {
        return http_response(&WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire());
    };
    let mut state = lock(state);
    state.parts = split_response(&result);
    http_response(&state.parts.len().to_string())
}

#[cfg(test)]
mod tests;
