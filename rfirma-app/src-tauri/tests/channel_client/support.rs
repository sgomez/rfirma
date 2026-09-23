//! Andamiaje compartido por `channel_client.rs` y `channel_operations.rs`: el canal sobre un puerto efímero y el cliente `wss://` que le habla.

// Cada fichero usa un subconjunto distinto: no todos se usan en todos.
#![allow(dead_code)]

pub use std::time::Duration;

pub use futures_util::{SinkExt, StreamExt};
pub use native_tls::{Certificate, TlsConnector};
pub use rfirma_lib::site::adapters::channel::{serve, SiteOperations};
pub use rfirma_lib::site::adapters::tls::LocalServerCertificate;
pub use rfirma_lib::site::domain::channel::{ChannelDuty, OpenChannel};
pub use rfirma_lib::site::domain::local_ca::LocalCa;
pub use rfirma_lib::site::domain::protocol::{
    AfirmaUrl, ChannelCredential, NegotiatedCredential, SafCode,
};
pub use rfirma_lib::site::ports::ReplyHandle;
pub use tokio_tungstenite::tungstenite::client::IntoClientRequest;
pub use tokio_tungstenite::tungstenite::Message;
pub use tokio_tungstenite::{Connector, MaybeTlsStream};

/// La credencial que la sede sortea: veinte alfanuméricos.
pub const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Lo que tarda de más una respuesta que no va a llegar.
pub const PATIENCE: Duration = Duration::from_secs(10);

/// Canal levantado sobre un puerto efímero.
pub struct AChannel {
    pub channel: OpenChannel,
    pub ca_pem: Vec<u8>,
}

impl AChannel {
    /// Levanta el canal para ese cometido, sobre un puerto que da el sistema.
    pub async fn serving(duty: ChannelDuty) -> Self {
        Self::serving_with(duty, no_operations()).await
    }

    /// Levanta el canal con el trámite doblado.
    pub async fn serving_with(duty: ChannelDuty, operations: SiteOperations) -> Self {
        let ca = LocalCa::generate().expect("la CA local deberia generarse");
        let certificate =
            LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("el sistema deberia dar un puerto efimero");

        let channel = serve(listener.into(), &certificate, duty, operations)
            .await
            .expect("el canal deberia levantarse");

        Self {
            channel,
            ca_pem: ca.certificate_pem().expect("la CA local en PEM"),
        }
    }

    /// El canal que sirve la conversación con la credencial de siempre.
    pub async fn serving_the_echo() -> Self {
        Self::serving(ChannelDuty::Serve(NegotiatedCredential::Required(
            ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
        )))
        .await
    }

    pub fn port(&self) -> u16 {
        self.channel.port()
    }

    /// El cliente de canal: un `wss://` que confía en esta CA local y nada más.
    pub async fn a_client(&self) -> ChannelClient {
        ChannelClient::connect(self.port(), Some(&self.ca_pem)).await
    }
}

/// Cliente de canal que habla `wss://` contra el servidor local.
pub struct ChannelClient {
    pub socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl ChannelClient {
    /// Conexión TLS y `Upgrade` de WebSocket encima.
    pub async fn try_connect(port: u16, ca_pem: Option<&[u8]>) -> Result<Self, String> {
        let mut builder = TlsConnector::builder();
        if let Some(ca_pem) = ca_pem {
            builder.add_root_certificate(
                Certificate::from_pem(ca_pem).expect("la CA local deberia leerse"),
            );
        }
        let connector = builder.build().expect("el conector deberia construirse");

        let request = format!("wss://localhost:{port}/")
            .into_client_request()
            .expect("la URL del canal deberia ser una peticion");

        let connected = tokio::time::timeout(
            PATIENCE,
            tokio_tungstenite::connect_async_tls_with_config(
                request,
                None,
                false,
                Some(Connector::NativeTls(connector)),
            ),
        )
        .await
        .map_err(|_| "el saludo no termino".to_owned())?;

        match connected {
            Ok((socket, _)) => Ok(Self { socket }),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Contra una dirección concreta, verificándola en la SAN como IP y no como nombre.
    pub async fn try_connect_to_the_address(
        address: &str,
        port: u16,
        ca_pem: &[u8],
    ) -> Result<Self, String> {
        let mut builder = TlsConnector::builder();
        builder.add_root_certificate(
            Certificate::from_pem(ca_pem).expect("la CA local deberia leerse"),
        );
        let connector = tokio_native_tls::TlsConnector::from(
            builder.build().expect("el conector deberia construirse"),
        );
        let tcp = tokio::net::TcpStream::connect((address, port))
            .await
            .map_err(|error| error.to_string())?;
        let tls = connector
            .connect(address, tcp)
            .await
            .map_err(|error| error.to_string())?;
        let host = if address.contains(':') {
            format!("[{address}]")
        } else {
            address.to_owned()
        };
        let request = format!("wss://{host}:{port}/")
            .into_client_request()
            .expect("la URL del canal deberia ser una peticion");
        let (socket, _) = tokio_tungstenite::client_async(request, MaybeTlsStream::NativeTls(tls))
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self { socket })
    }

    pub async fn connect(port: u16, ca_pem: Option<&[u8]>) -> Self {
        Self::try_connect(port, ca_pem)
            .await
            .expect("el saludo deberia terminar bien")
    }

    /// Manda un mensaje y espera la respuesta.
    pub async fn say(&mut self, message: &str) -> Option<String> {
        self.socket
            .send(Message::text(message.to_owned()))
            .await
            .expect("el canal deberia aceptar el mensaje");

        loop {
            let received = tokio::time::timeout(PATIENCE, self.socket.next())
                .await
                .expect("la respuesta deberia llegar")?;
            match received.expect("la respuesta deberia leerse") {
                Message::Text(text) => return Some(text.as_str().to_owned()),
                Message::Close(_) => return None,
                _ => continue,
            }
        }
    }

    /// El eco tal y como lo manda el cliente publicado.
    pub async fn echo(&mut self, credential: &str) -> Option<String> {
        self.say(&format!("echo=-idsession={credential}@EOF")).await
    }

    /// ¿Sigue abierto el canal después de la respuesta?
    pub async fn is_still_open(&mut self) -> bool {
        match tokio::time::timeout(Duration::from_millis(200), self.socket.next()).await {
            Err(_) => true,
            Ok(None) => false,
            Ok(Some(received)) => !matches!(received, Ok(Message::Close(_)) | Err(_)),
        }
    }
}

/// Trámite que no contesta las operaciones recibidas.
pub fn no_operations() -> SiteOperations {
    SiteOperations::for_operations(|_, _| {})
}

/// El cometido de servir con la credencial de siempre.
pub fn serving_the_credential() -> ChannelDuty {
    ChannelDuty::Serve(NegotiatedCredential::Required(
        ChannelCredential::parse(CREDENTIAL).expect("credencial"),
    ))
}

pub fn an_operation(verb: &str) -> String {
    format!("afirma://{verb}?op={verb}&idsession={CREDENTIAL}")
}

/// Trámite que contesta en el acto cada operación con su verbo.
pub fn answering_each_operation() -> SiteOperations {
    SiteOperations::for_operations(|url: AfirmaUrl, reply: ReplyHandle| {
        let _ = reply.answer(format!("contestada:{}", url.verb()));
    })
}

/// Espera, sin pasarse, a que la cuenta llegue a lo esperado.
pub async fn counted(count: &std::sync::atomic::AtomicUsize, expected: usize) -> usize {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        let now = count.load(std::sync::atomic::Ordering::SeqCst);
        if now == expected || tokio::time::Instant::now() >= deadline {
            return now;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
