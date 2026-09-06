//! Servidor WebSocket local sobre TLS para operaciones con la sede (ADR-0005).

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::oneshot;
use tokio_native_tls::native_tls::{Identity, TlsAcceptor as NativeTlsAcceptor};
use tokio_native_tls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

use crate::channel::conversation::{answer, Answer, ChannelDuty};
use crate::channel::error::{ChannelError, Situation};
use crate::channel::reply::ReplyHandle;
use crate::protocol::AfirmaUrl;
use crate::tls::LocalServerCertificate;

/// Manejador que atiende la operación recibida por el canal.
pub type SiteOperations = Arc<dyn Fn(AfirmaUrl, ReplyHandle) + Send + Sync>;

/// Canal abierto con su puerto de escucha y asa de cierre.
pub struct OpenChannel {
    port: u16,
    shutdown: Shutdown,
}

impl OpenChannel {
    /// Crea un canal abierto con su puerto y asa de cierre.
    pub fn new(port: u16, shutdown: Shutdown) -> Self {
        Self { port, shutdown }
    }

    /// Puerto en el que escucha el canal.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Cierra el canal y deja de escuchar conexiones.
    pub fn close(self) {
        self.shutdown.now();
    }
}

impl std::fmt::Debug for OpenChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenChannel")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// Asa para apagar el servidor del canal.
pub struct Shutdown(Box<dyn FnOnce() + Send>);

impl Shutdown {
    /// Construye un asa de apagado a partir de una clausura.
    pub fn of(closing: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(closing))
    }

    /// Ejecuta el apagado del servidor.
    pub fn now(self) {
        (self.0)();
    }
}

/// Inicia la escucha del canal sobre un listener ya enlazado.
pub async fn serve(
    listener: TcpListener,
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    operations: SiteOperations,
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

/// Enlaza el primer puerto disponible y arranca el servidor del canal.
pub fn open(
    ports: &[u16],
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    operations: SiteOperations,
) -> Result<OpenChannel, ChannelError> {
    let listener = crate::channel::bind::bind_first_free(ports)?;
    tauri::async_runtime::block_on(serve(listener, certificate, duty, operations))
}

fn acceptor_for(certificate: &LocalServerCertificate) -> Result<TlsAcceptor, ChannelError> {
    let material = |error: String| ChannelError::new(Situation::MaterialNotUsable, error);

    let identity = Identity::from_pkcs8(
        &certificate
            .certificate_pem()
            .map_err(|error| material(error.to_string()))?,
        &certificate
            .private_key_pem()
            .map_err(|error| material(error.to_string()))?,
    )
    .map_err(|error| material(error.to_string()))?;

    let acceptor = NativeTlsAcceptor::new(identity).map_err(|error| material(error.to_string()))?;

    Ok(TlsAcceptor::from(acceptor))
}

async fn accept_until_stopped(
    listener: tokio::net::TcpListener,
    acceptor: Arc<TlsAcceptor>,
    duty: ChannelDuty,
    operations: SiteOperations,
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
                let operations = Arc::clone(&operations);
                tokio::spawn(async move {
                    let _ = attend(stream, peer, &acceptor, &duty, &operations).await;
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
    operations: &SiteOperations,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let from_loopback = peer.ip().is_loopback();
    let encrypted = acceptor.accept(stream).await?;
    let mut socket = tokio_tungstenite::accept_async(encrypted).await?;

    while let Some(received) = socket.next().await {
        let text = match received? {
            Message::Text(text) => text.as_str().to_owned(),
            Message::Binary(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Message::Close(_) => break,
            _ => continue,
        };

        match answer(duty, from_loopback, &text) {
            Answer::Reply(reply) => socket.send(Message::text(reply)).await?,
            Answer::ReplyAndClose(reply) => {
                socket.send(Message::text(reply)).await?;
                socket.close(None).await?;
                break;
            }
            Answer::Pending(url) => {
                let (sender, receiver) = oneshot::channel();
                operations(url, ReplyHandle::of(sender));
                if let Ok(reply) = receiver.await {
                    socket.send(Message::text(reply)).await?;
                }
                socket.close(None).await?;
                break;
            }
        }
    }

    Ok(())
}
