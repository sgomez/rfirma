//! **El servidor del canal**: WebSocket sobre TLS en `127.0.0.1` (ID-212,
//! ID-213).
//!
//! # No existe escuchador en claro
//!
//! Rechazar `ws://` no es una comprobación que haya que escribir: es una ruta
//! que no hay. Todo lo que se acepta aquí pasa por [`tokio_native_tls`] antes
//! de llegar al saludo WebSocket, así que un cliente que hable en claro se
//! queda en el saludo TLS y no ve un servidor de protocolo al otro lado.
//!
//! Por eso [`tokio_tungstenite`] entra **sin ninguna característica de TLS**:
//! las suyas son de cliente, y `accept_async` recibe el *stream* ya cifrado.
//!
//! # No posee su runtime
//!
//! [`serve`] es una `async fn` que recibe el escuchador **ya enlazado**
//! ([`crate::channel::bind`]) y devuelve el puerto y un asa de apagado
//! ([`OpenChannel`]). En producción corre sobre `tauri::async_runtime`; en
//! pruebas, con `#[tokio::test]`. Eso es lo que permite atar un puerto efímero
//! desde una prueba sin montar la aplicación.
//!
//! # El original es oráculo, no código prestado
//!
//! `AfirmaWebSocketServer` y compañía se leen para saber **qué** hace el
//! servidor —el orden de las guardias, el `OK` del eco, el formato del
//! error—; no se traducen (ID-219). Lo que hace cada mensaje está en
//! [`crate::channel::conversation`], sin socket delante.

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

/// **Quién atiende la operación que llegó por el canal** (ID-330).
///
/// El servidor no sabe qué es un trámite: cuando la conversación dice que el
/// mensaje es una operación legítima ([`Answer::Pending`]), se la entrega a
/// esto junto con el asa por la que se contesta, y se queda esperando. Lo
/// cumple el adaptador, que es quien puede armar el escritorio del trámite
/// desde el estado de la aplicación.
///
/// Es `Arc` y no una referencia porque cada conexión se atiende en su propia
/// tarea del runtime, que vive más que la llamada que abrió el canal.
pub type SiteOperations = Arc<dyn Fn(AfirmaUrl, ReplyHandle) + Send + Sync>;

/// **Un canal abierto**: en qué puerto quedó y cómo se apaga.
///
/// Quien lo recibe es responsable de tenerlo vivo: soltarlo no apaga nada por
/// sí solo, pero perder el asa deja el canal escuchando hasta que muera el
/// proceso.
pub struct OpenChannel {
    port: u16,
    shutdown: Shutdown,
}

impl OpenChannel {
    /// Un canal abierto en ese puerto, que se apaga con esa asa.
    pub fn new(port: u16, shutdown: Shutdown) -> Self {
        Self { port, shutdown }
    }

    /// El puerto en el que quedó escuchando: uno de los que sorteó la sede.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Deja de escuchar. Las conversaciones en curso terminan; no se acepta
    /// ninguna nueva.
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

/// **El asa de apagado**: una función y nada más.
///
/// Es un cierre y no un `JoinHandle` ni un `Sender` para que el puerto de
/// transporte del caso de uso ([`crate::app::site`]) no tenga que nombrar a
/// `tokio`: una prueba dobla el transporte con un asa que no apaga nada.
pub struct Shutdown(Box<dyn FnOnce() + Send>);

impl Shutdown {
    /// El asa que ejecuta eso al apagar.
    pub fn of(closing: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(closing))
    }

    /// Apaga.
    pub fn now(self) {
        (self.0)();
    }
}

/// Levanta el canal sobre un escuchador ya enlazado.
///
/// Vuelve en cuanto está escuchando: la aceptación de conexiones se queda en
/// una tarea del runtime que llame, y termina cuando se cierra
/// [`OpenChannel`].
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

/// **Ata y sirve**: los dos pasos del canal, para quien tiene un runtime de
/// Tauri detrás.
///
/// Es lo que cumple el puerto de transporte del caso de uso
/// ([`crate::app::site::ChannelTransport`]) en producción. Corre sobre
/// `tauri::async_runtime` —el runtime de la aplicación, que sobrevive a esta
/// llamada— y no sobre uno propio: un runtime creado y soltado aquí se
/// llevaría por delante la tarea que acepta conexiones (ID-213).
pub fn open(
    ports: &[u16],
    certificate: &LocalServerCertificate,
    duty: ChannelDuty,
    operations: SiteOperations,
) -> Result<OpenChannel, ChannelError> {
    let listener = crate::channel::bind::bind_first_free(ports)?;
    tauri::async_runtime::block_on(serve(listener, certificate, duty, operations))
}

/// El certificado del servidor local, envuelto en lo que la pila de TLS acepta.
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

/// Acepta conexiones hasta que se cierre el canal.
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
                // Un `accept` que falla en el *loopback* es un transitorio
                // —`EMFILE`, `ENFILE`, `ECONNABORTED`—, no el final del canal:
                // romper el bucle aquí dejaría a rFirma sin escuchar en
                // silencio, sin `ChannelError` y sin que `OpenChannel` se
                // entere. Se descarta la conexión y se sigue atendiendo; el
                // canal sólo se apaga por su asa.
                let Ok((stream, peer)) = accepted else { continue };
                let acceptor = Arc::clone(&acceptor);
                let duty = duty.clone();
                let operations = Arc::clone(&operations);
                tokio::spawn(async move {
                    // Una conexión que se cae —un saludo TLS que no cuadra, una
                    // sede que cierra— no se lleva el canal por delante: la
                    // siguiente invocación vuelve a llamar.
                    let _ = attend(stream, peer, &acceptor, &duty, &operations).await;
                });
            }
        }
    }
}

/// Una conversación entera sobre una conexión: saludo TLS, saludo WebSocket y
/// los mensajes.
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
            // **La operación queda pendiente** (ID-320): no se escribe nada y
            // la conexión se queda abierta hasta que el trámite entregue la
            // respuesta, que es lo que la sede espera mientras la persona
            // decide. Que el asa se suelte sin contestar —un trámite que se fue
            // sin responder— cierra el canal sin escribir: la sede se queda con
            // su propio plazo y no con una línea inventada aquí (ID-322).
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
