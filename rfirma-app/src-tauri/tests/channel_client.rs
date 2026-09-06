//! Cliente de canal para probar el canal local de punta a punta (ADR-0005).

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use native_tls::{Certificate, TlsConnector};
use rfirma_lib::app::errand::LiveErrand;
use rfirma_lib::app::site::Attendance;
use rfirma_lib::app::startup::{attend_site_launch, LocalCaReach};
use rfirma_lib::channel::{
    bind_first_free, serve, ChannelDuty, OpenChannel, ReplyHandle, SiteOperations,
    THE_PORT_OF_THE_THIRD_PROTOCOL,
};
use rfirma_lib::protocol::{ChannelCredential, LaunchRequest, SafCode};
use rfirma_lib::tls::{LocalCa, LocalServerCertificate};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;

/// La credencial que la sede sortea: veinte alfanuméricos.
const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Lo que tarda de más una respuesta que no va a llegar.
const PATIENCE: Duration = Duration::from_secs(10);

/// Canal levantado sobre un puerto efímero.
struct AChannel {
    channel: OpenChannel,
    ca_pem: Vec<u8>,
}

impl AChannel {
    /// Levanta el canal para ese cometido, sobre un puerto que da el sistema.
    async fn serving(duty: ChannelDuty) -> Self {
        Self::serving_with(duty, no_operations()).await
    }

    /// Levanta el canal con el trámite doblado.
    async fn serving_with(duty: ChannelDuty, operations: SiteOperations) -> Self {
        let ca = LocalCa::generate().expect("la CA local deberia generarse");
        let certificate =
            LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .expect("el sistema deberia dar un puerto efimero");

        let channel = serve(listener, &certificate, duty, operations)
            .await
            .expect("el canal deberia levantarse");

        Self {
            channel,
            ca_pem: ca.certificate_pem().expect("la CA local en PEM"),
        }
    }

    /// El canal que sirve la conversación con la credencial de siempre.
    async fn serving_the_echo() -> Self {
        Self::serving(ChannelDuty::Serve(
            ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
        ))
        .await
    }

    fn port(&self) -> u16 {
        self.channel.port()
    }

    /// El cliente de canal: un `wss://` que confía en esta CA local y nada más.
    async fn a_client(&self) -> ChannelClient {
        ChannelClient::connect(self.port(), Some(&self.ca_pem)).await
    }
}

/// Cliente de canal que habla `wss://` contra el servidor local.
struct ChannelClient {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl ChannelClient {
    /// Conexión TLS y `Upgrade` de WebSocket encima.
    async fn try_connect(port: u16, ca_pem: Option<&[u8]>) -> Result<Self, String> {
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

    async fn connect(port: u16, ca_pem: Option<&[u8]>) -> Self {
        Self::try_connect(port, ca_pem)
            .await
            .expect("el saludo deberia terminar bien")
    }

    /// Manda un mensaje y espera la respuesta.
    async fn say(&mut self, message: &str) -> Option<String> {
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
    async fn echo(&mut self, credential: &str) -> Option<String> {
        self.say(&format!("echo=-idsession={credential}@EOF")).await
    }

    /// ¿Sigue abierto el canal después de la respuesta?
    async fn is_still_open(&mut self) -> bool {
        match tokio::time::timeout(Duration::from_millis(200), self.socket.next()).await {
            Err(_) => true,
            Ok(None) => false,
            Ok(Some(received)) => !matches!(received, Ok(Message::Close(_)) | Err(_)),
        }
    }
}

#[tokio::test]
async fn the_channel_answers_the_echo_with_ok_over_tls() {
    let canal = AChannel::serving_the_echo().await;

    let mut client = canal.a_client().await;

    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
    assert!(
        client.is_still_open().await,
        "contestado el eco, la sede manda la operacion por el mismo canal"
    );
}

#[tokio::test]
async fn a_client_that_speaks_in_the_clear_never_reaches_the_protocol() {
    let canal = AChannel::serving_the_echo().await;
    let port = canal.port();

    let request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .expect("la URL deberia ser una peticion");
    let in_the_clear = tokio::time::timeout(PATIENCE, tokio_tungstenite::connect_async(request))
        .await
        .expect("el intento deberia terminar");

    assert!(
        in_the_clear.is_err(),
        "un `ws://` que llegue al protocolo seria una segunda puerta sin cifrar"
    );
}

#[tokio::test]
async fn a_client_that_does_not_trust_the_local_ca_is_turned_away_at_the_handshake() {
    let canal = AChannel::serving_the_echo().await;

    let without_the_ca = ChannelClient::try_connect(canal.port(), None).await;

    assert!(
        without_the_ca.is_err(),
        "sin la CA local en el almacen, el navegador no abre el canal"
    );
}

#[tokio::test]
async fn an_echo_with_another_credential_is_refused_and_the_channel_closes() {
    let canal = AChannel::serving_the_echo().await;
    let mut intruder = canal.a_client().await;

    let answer = intruder.echo("otraPaginaDelEquipo0").await;

    assert_eq!(
        answer,
        Some("SAF_46: Id de sesion invalido; el parametro que falla es 'idsession'".to_owned())
    );
    assert!(
        !intruder.is_still_open().await,
        "el canal se cierra detras del rechazo"
    );
}

#[tokio::test]
async fn a_launch_with_an_unsupported_version_is_refused_over_the_socket() {
    let refusal = LaunchRequest::parse(&format!(
        "afirma://websocket?ports=0&v=3&idsession={CREDENTIAL}"
    ))
    .expect_err("la version 3 no se habla aqui");
    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);

    let canal = AChannel::serving(ChannelDuty::Refuse(refusal.answer())).await;
    let mut client = canal.a_client().await;

    let answer = client.echo(CREDENTIAL).await;

    assert_eq!(
        answer,
        Some("SAF_21: Este tramite no es compatible con la version instalada".to_owned())
    );
    assert!(
        !client.is_still_open().await,
        "ese canal no expone ninguna capacidad: contesta y cierra"
    );
}

#[tokio::test]
async fn the_channel_ends_up_on_one_of_the_ports_the_site_drew() {
    let drawn = {
        let mut ports = Vec::new();
        for _ in 0..3 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("puerto efimero");
            ports.push(listener.local_addr().expect("atado").port());
        }
        ports
    };

    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
    let listener = bind_first_free(&drawn).expect("alguno de los tres deberia estar libre");

    let channel = serve(
        listener,
        &certificate,
        ChannelDuty::Serve(ChannelCredential::parse(CREDENTIAL).expect("credencial")),
        no_operations(),
    )
    .await
    .expect("el canal deberia levantarse");

    assert!(
        drawn.contains(&channel.port()),
        "el canal quedo en {}, que la sede no sorteo",
        channel.port()
    );

    let mut client = ChannelClient::connect(
        channel.port(),
        Some(&ca.certificate_pem().expect("la CA local en PEM")),
    )
    .await;
    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
}

#[tokio::test]
async fn once_closed_the_channel_accepts_no_new_conversations() {
    let canal = AChannel::serving_the_echo().await;
    let port = canal.port();
    let ca_pem = canal.ca_pem.clone();
    canal.channel.close();

    let mut refused = false;
    for _ in 0..40 {
        if ChannelClient::try_connect(port, Some(&ca_pem))
            .await
            .is_err()
        {
            refused = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(refused, "el canal cerrado seguia aceptando conversaciones");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_site_launch_ends_with_the_echo_answered_over_the_open_channel() {
    let free = {
        let mut ports = Vec::new();
        for _ in 0..2 {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("puerto efimero");
            ports.push(listener.local_addr().expect("atado").port());
        }
        ports
    };
    let drawn = [THE_PORT_OF_THE_THIRD_PROTOCOL, free[0], free[1]];

    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let ca_pem = ca.certificate_pem().expect("la CA local en PEM");
    let certificate =
        LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");

    let windows = std::sync::atomic::AtomicUsize::new(0);
    let live = LiveErrand::default();
    let url = format!(
        "afirma://websocket?ports={},{},{}&v=4&idsession={CREDENTIAL}",
        drawn[0], drawn[1], drawn[2]
    );

    let attendance = attend_site_launch(
        &url,
        &|ports, duty| {
            let listener = bind_first_free(ports)?;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(serve(
                    listener,
                    &certificate,
                    duty,
                    no_operations(),
                ))
            })
        },
        &|_| {
            windows.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        },
        &live,
        LocalCaReach::NotAnObstacle,
    );

    let Attendance::Serving { channel, .. } = &attendance else {
        panic!("la invocacion era buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        free[0],
        "el canal queda en el primero libre que sorteo la sede, y nunca en el 63117"
    );
    assert_eq!(
        windows.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "con trámite hay una ventana de sede, y una sola (ID-334)"
    );

    let mut client = ChannelClient::connect(channel.port(), Some(&ca_pem)).await;

    assert_eq!(client.echo(CREDENTIAL).await, Some("OK".to_owned()));
}

/// Trámite que no contesta las operaciones recibidas.
fn no_operations() -> SiteOperations {
    std::sync::Arc::new(|_, _| {})
}

#[tokio::test(flavor = "multi_thread")]
async fn an_operation_is_answered_by_the_errand_and_not_by_the_channel() {
    let held: std::sync::Arc<std::sync::Mutex<Option<ReplyHandle>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    let keeping = std::sync::Arc::clone(&held);

    let channel = AChannel::serving_with(
        ChannelDuty::Serve(ChannelCredential::parse(CREDENTIAL).expect("credencial")),
        std::sync::Arc::new(move |url, reply| {
            assert_eq!(url.verb(), "selectcert");
            *keeping.lock().expect("el candado") = Some(reply);
        }),
    )
    .await;
    let mut client = channel.a_client().await;

    assert_eq!(client.echo(CREDENTIAL).await.as_deref(), Some("OK"));

    let operation = format!("afirma://selectcert?op=selectcert&idsession={CREDENTIAL}");
    client
        .socket
        .send(Message::text(operation))
        .await
        .expect("la operacion deberia salir");

    let waited = tokio::time::timeout(Duration::from_millis(300), client.socket.next()).await;
    assert!(
        waited.is_err(),
        "la operacion no se contesta hasta que lo haga el tramite: {waited:?}"
    );

    let reply = held
        .lock()
        .expect("el candado")
        .take()
        .expect("el tramite recibio el asa");
    reply.answer("CANCEL".to_owned());

    let answered = tokio::time::timeout(PATIENCE, client.socket.next())
        .await
        .expect("la respuesta del tramite deberia llegar")
        .expect("hay mensaje")
        .expect("y se lee");
    assert_eq!(answered.into_text().expect("es texto").as_str(), "CANCEL");
}
