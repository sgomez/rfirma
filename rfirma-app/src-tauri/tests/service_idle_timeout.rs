//! Mide si el transporte `service` sigue vivo tras una conexión callada larga (issue #499).
//!
//! Ignorada por defecto: duerme de verdad más de 240s, así que no corre en `just check`. Se
//! ejecuta a mano con `cargo test --test service_idle_timeout -- --ignored --nocapture`.

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;
use native_tls::{Certificate, TlsConnector};
use rfirma_lib::site::adapters::service::RawTlsService;
use rfirma_lib::site::adapters::tls::{CaFiles, LocalCaStore};
use rfirma_lib::site::application::errand::Transport;
use rfirma_lib::site::domain::channel::{ChannelDuty, ChannelLocation};
use rfirma_lib::site::domain::local_ca::LocalCa;
use rfirma_lib::site::domain::protocol::{ChannelCredential, NegotiatedCredential};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Más que el máximo de 240s que el original le daba a un lote (`setConnectionLostTimeout`).
const LONGER_THAN_THE_ORIGINAL_BATCH_ALLOWANCE: Duration = Duration::from_secs(245);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "duerme 245s de verdad: mide el timeout real del transporte service (issue #499)"]
async fn a_silent_service_connection_survives_longer_than_the_original_batch_allowance() {
    let directory = tempfile::tempdir().expect("directorio temporal");
    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let ca_pem = ca.certificate_pem().expect("la CA local en PEM");
    let store = LocalCaStore::new(
        CaFiles::new(
            directory.path().join("ca.pem"),
            directory.path().join("ca.key"),
        ),
        CaFiles::new(
            directory.path().join("ca-next.pem"),
            directory.path().join("ca-next.key"),
        ),
    );
    store.write(&ca).expect("la CA deberia guardarse");

    let inbox = std::sync::Arc::new(
        |_url, reply: rfirma_lib::site::application::errand::ReplyHandle| {
            reply.answer("no se llama".to_owned());
        },
    );
    let transport = RawTlsService::new(store, inbox);
    let duty = ChannelDuty::Serve(NegotiatedCredential::Required(
        ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
    ));
    // `RawTlsService::open` bloquea su propio hilo con `block_on`: no cabe en el runtime de
    // este test, igual que `attend_site_launch` lo aisla en las pruebas del canal wss.
    let channel = tokio::task::spawn_blocking(move || {
        transport.open(&ChannelLocation::Service(vec![0]), duty)
    })
    .await
    .expect("el hilo bloqueante deberia terminar")
    .expect("el transporte service deberia levantarse");

    let mut builder = TlsConnector::builder();
    builder.add_root_certificate(Certificate::from_pem(&ca_pem).expect("la CA local en PEM"));
    let connector: tokio_native_tls::TlsConnector = builder
        .build()
        .expect("el conector deberia construirse")
        .into();

    let tcp = tokio::net::TcpStream::connect(("127.0.0.1", channel.port()))
        .await
        .expect("el socket deberia conectarse");
    let mut tls = connector
        .connect("localhost", tcp)
        .await
        .expect("el saludo TLS deberia terminar bien");

    // La conexión se queda abierta y callada, sin mandar nada, durante más de lo que el
    // original tolera para un lote grande antes de subir su timeout a 240s.
    tokio::time::sleep(LONGER_THAN_THE_ORIGINAL_BATCH_ALLOWANCE).await;

    tls.write_all(format!("echo=-idsession={CREDENTIAL}@EOF").as_bytes())
        .await
        .expect("el transporte service deberia seguir aceptando bytes tras el silencio");

    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(10), tls.read_to_end(&mut response))
        .await
        .expect("la respuesta deberia llegar")
        .expect("la respuesta deberia leerse");

    let text = String::from_utf8(response).expect("la respuesta es utf-8");
    let body = text.split("\n\n").nth(1).expect("la respuesta trae cuerpo");
    let decoded = URL_SAFE.decode(body).expect("el cuerpo es base64 valido");
    assert_eq!(
        String::from_utf8(decoded).expect("el cuerpo decodificado es utf-8"),
        "OK",
        "el eco deberia contestarse tras el silencio, sin timeout propio de por medio"
    );
}
