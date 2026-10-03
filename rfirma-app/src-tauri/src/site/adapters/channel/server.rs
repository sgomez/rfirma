//! Servidor WebSocket local sobre TLS para operaciones con la sede; no existe escuchador en claro (ADR-0005).

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::handshake::server::Request;
use tokio_tungstenite::tungstenite::Message;

use crate::site::adapters::channel::acceptor::{LocalTlsAcceptor, LocalTlsStream};
use crate::site::adapters::channel::bind::{LoopbackAcceptor, LoopbackListeners};
use crate::site::adapters::channel::conversation::{another_operation_in_flight, answer, Answer};
use crate::site::adapters::cookies::OperationCookies;
use crate::site::adapters::tls::LocalServerCertificate;
use crate::site::domain::channel::ChannelDuty;
use crate::site::domain::channel::{ChannelError, Situation};
use crate::site::domain::channel::{ChannelLocation, OpenChannel, Shutdown};
use crate::site::domain::site_origin::SiteOrigin;

use crate::site::ports::Inbox;

/// Manejador que atiende la operación recibida por el canal.
pub type SiteOperations = Inbox;

/// Inicia la escucha del canal sobre un listener ya enlazado.
pub async fn serve(
    listener: LoopbackListeners,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    operations: SiteOperations,
) -> Result<OpenChannel, ChannelError> {
    let not_listening =
        |error: std::io::Error| ChannelError::new(Situation::NotListening, error.to_string());
    let port = listener.port().map_err(not_listening)?;
    let acceptor = Arc::new(acceptor_for(certificate)?);
    let listener = listener.into_async().map_err(not_listening)?;

    let (stop, stopped) = oneshot::channel();
    tokio::spawn(accept_until_stopped(
        listener, acceptor, duty, operations, stopped,
    ));

    Ok(OpenChannel::new(
        port,
        Shutdown::of(move || {
            let _ = stop.send(());
        }),
    ))
}

/// Enlaza la ubicación indicada y arranca el servidor del canal.
pub fn open(
    location: &ChannelLocation,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    operations: SiteOperations,
) -> Result<OpenChannel, ChannelError> {
    let listener = crate::site::adapters::channel::bind::bind_first_free(location)?;
    tauri::async_runtime::block_on(serve(listener, certificate, duty, operations))
}

/// El aceptador TLS del material del servidor local, compartido con el transporte `service`.
pub(crate) fn acceptor_for(
    certificate: &LocalServerCertificate,
) -> Result<LocalTlsAcceptor, ChannelError> {
    let material = |error: String| ChannelError::new(Situation::MaterialNotUsable, error);

    LocalTlsAcceptor::from_pem(
        &certificate
            .certificate_pem()
            .map_err(|error| material(error.to_string()))?,
        &certificate
            .private_key_pem()
            .map_err(|error| material(error.to_string()))?,
    )
    .map_err(material)
}

async fn accept_until_stopped(
    listener: LoopbackAcceptor,
    acceptor: Arc<LocalTlsAcceptor>,
    duty: ChannelDuty,
    operations: SiteOperations,
    stopped: oneshot::Receiver<()>,
) {
    tokio::pin!(stopped);
    let clients = Clients::default();

    loop {
        tokio::select! {
            _ = &mut stopped => break,
            accepted = listener.accept() => {
                let Ok((stream, peer)) = accepted else { continue };
                let acceptor = Arc::clone(&acceptor);
                let duty = duty.clone();
                let operations = operations.clone();
                let clients = clients.clone();
                tokio::spawn(async move {
                    let _ = attend(stream, peer, &acceptor, &duty, &operations, &clients).await;
                });
            }
        }
    }
}

/// Lo que comparten las conexiones de un canal: el primer cliente y la operación en vuelo.
#[derive(Clone, Default)]
struct Clients {
    first_taken: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

impl Clients {
    fn is_the_first(&self) -> bool {
        !self.first_taken.swap(true, Ordering::SeqCst)
    }

    fn take_the_turn(&self) -> Option<Turn> {
        (!self.busy.swap(true, Ordering::SeqCst)).then(|| Turn(Arc::clone(&self.busy)))
    }
}

/// La operación en vuelo del canal; se libera al soltarla.
struct Turn(Arc<AtomicBool>);

impl Drop for Turn {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Avisa al soltarse de que se ha ido el primer cliente, salga como salga su conexión.
struct FirstClient(SiteOperations);

impl Drop for FirstClient {
    fn drop(&mut self) {
        self.0.first_client_left();
    }
}

type Socket = tokio_tungstenite::WebSocketStream<LocalTlsStream>;

enum Waited {
    Reply(String),
    NoReply,
    ClientGone,
}

async fn next_text(
    socket: &mut Socket,
) -> Result<Option<String>, tokio_tungstenite::tungstenite::Error> {
    while let Some(received) = socket.next().await {
        match received? {
            Message::Text(text) => return Ok(Some(text.as_str().to_owned())),
            Message::Binary(bytes) => {
                return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
            }
            Message::Close(_) => return Ok(None),
            _ => continue,
        }
    }
    Ok(None)
}

async fn wait_for_the_reply(
    socket: &mut Socket,
    mut reply: oneshot::Receiver<String>,
    queued: &mut VecDeque<String>,
) -> Waited {
    loop {
        tokio::select! {
            biased;
            answered = &mut reply => {
                return answered.map_or(Waited::NoReply, Waited::Reply);
            }
            received = next_text(socket) => match received {
                Ok(Some(text)) => queued.push_back(text),
                Ok(None) | Err(_) => return Waited::ClientGone,
            }
        }
    }
}

/// Acepta el saludo TLS ya cifrado, tomando el `Origin` que trae; nunca lo rechaza por su valor.
#[allow(clippy::result_large_err)]
async fn accept_with_the_origin_of_the_greeting(
    encrypted: LocalTlsStream,
) -> Result<(Socket, SiteOrigin), tokio_tungstenite::tungstenite::Error> {
    let mut origin = SiteOrigin::absent();
    let socket = tokio_tungstenite::accept_hdr_async(encrypted, |request: &Request, response| {
        origin = SiteOrigin::from_header(
            request
                .headers()
                .get("origin")
                .and_then(|value| value.to_str().ok()),
        );
        Ok(response)
    })
    .await?;
    Ok((socket, origin))
}

async fn attend(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    acceptor: &LocalTlsAcceptor,
    duty: &ChannelDuty,
    operations: &SiteOperations,
    clients: &Clients,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let from_loopback = peer.ip().is_loopback();
    let encrypted = acceptor.accept(stream).await?;
    let (mut socket, origin) = accept_with_the_origin_of_the_greeting(encrypted).await?;
    let _first = clients
        .is_the_first()
        .then(|| FirstClient(operations.clone()));
    let mut queued = VecDeque::new();

    loop {
        let text = match queued.pop_front() {
            Some(text) => text,
            None => match next_text(&mut socket).await? {
                Some(text) => text,
                None => break,
            },
        };

        match answer(duty, from_loopback, &text) {
            Answer::Reply(reply) => {
                operations.arrived();
                socket.send(Message::text(reply)).await?;
            }
            Answer::Refuse(reply) => {
                socket.send(Message::text(reply)).await?;
            }
            Answer::ReplyAndClose(reply) => {
                let sent = socket.send(Message::text(reply)).await;
                if sent.is_ok() && matches!(duty, ChannelDuty::Refuse(_)) {
                    operations.arrived();
                }
                socket.close(None).await?;
                sent?;
                break;
            }
            Answer::Pending(url) => {
                operations.arrived();
                let Some(_turn) = clients.take_the_turn() else {
                    socket
                        .send(Message::text(another_operation_in_flight()))
                        .await?;
                    continue;
                };
                let (sender, receiver) = oneshot::channel();
                let (acknowledged, acknowledgement) = crate::site::ports::Acknowledgement::pair();
                let reply = crate::site::ports::ReplyHandle::of(move |text| {
                    OperationCookies::of_the_process().forget();
                    let _ = sender.send(text);
                    acknowledgement
                });
                let operations = operations.clone();
                let origin = origin.clone();
                // Fuera del hilo del socket: lo que el token deja en la cola de errores de
                // OpenSSL de su hilo haría fallar la siguiente lectura TLS del canal.
                tokio::task::spawn_blocking(move || operations.deliver(url, origin, reply));
                match wait_for_the_reply(&mut socket, receiver, &mut queued).await {
                    Waited::Reply(reply) => {
                        let sent = socket.send(Message::text(reply)).await;
                        if sent.is_ok() {
                            acknowledged.fulfil();
                        }
                        sent?;
                    }
                    Waited::NoReply => {}
                    Waited::ClientGone => break,
                }
            }
        }
    }

    Ok(())
}
