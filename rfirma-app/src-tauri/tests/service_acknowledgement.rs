//! Cliente real sobre loopback para el transporte `service` (TD-10, issue #683).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE;
use base64::Engine as _;
use native_tls::{Certificate, TlsConnector};
use rfirma_lib::site::adapters::service::RawTlsService;
use rfirma_lib::site::adapters::tls::{CaFiles, LocalCaStore};
use rfirma_lib::site::application::errand::Transport;
use rfirma_lib::site::domain::channel::{ChannelDuty, ChannelLocation};
use rfirma_lib::site::domain::local_ca::LocalCa;
use rfirma_lib::site::domain::protocol::{AfirmaUrl, ChannelCredential, NegotiatedCredential};
use rfirma_lib::site::ports::{Inbox, ReplyHandle};
use tokio::io::AsyncWriteExt;

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

/// Conexion TLS cruda con `SO_LINGER` a cero: al cerrarla el sistema manda un RST en vez de un
/// cierre ordenado, igual que exige la prueba equivalente del websocket con un cliente ya ido.
async fn a_linger_free_connection(
    port: u16,
    ca_pem: &[u8],
) -> tokio_native_tls::TlsStream<tokio::net::TcpStream> {
    let mut builder = TlsConnector::builder();
    builder.add_root_certificate(Certificate::from_pem(ca_pem).expect("la CA local en PEM"));
    let connector: tokio_native_tls::TlsConnector = builder
        .build()
        .expect("el conector deberia construirse")
        .into();

    let tcp = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("el socket deberia conectarse");
    // `std::net::TcpStream::set_linger` sigue inestable (rust#88494): usamos la de tokio, que
    // solo bloquearia el hilo en el cierre con un plazo mayor que cero.
    #[allow(deprecated)]
    tcp.set_linger(Some(Duration::ZERO))
        .expect("SO_LINGER deberia aceptarse");

    connector
        .connect("localhost", tcp)
        .await
        .expect("el saludo TLS deberia terminar bien")
}

#[tokio::test(flavor = "multi_thread")]
async fn the_acknowledgement_is_not_fulfilled_for_a_service_client_already_gone() {
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

    let held: Arc<Mutex<Option<ReplyHandle>>> = Arc::new(Mutex::new(None));
    let keeping = Arc::clone(&held);
    let inbox = Inbox::for_operations(move |_url: AfirmaUrl, reply: ReplyHandle| {
        *keeping.lock().expect("el candado") = Some(reply);
    });
    let transport = RawTlsService::new(store, inbox);
    let duty = ChannelDuty::Serve(NegotiatedCredential::Required(
        ChannelCredential::parse(CREDENTIAL).expect("veinte alfanumericos son credencial"),
    ));
    // `RawTlsService::open` bloquea su propio hilo con `block_on`, igual que en la medicion del
    // timeout de inactividad de este mismo transporte.
    let channel = tokio::task::spawn_blocking(move || {
        transport.open(&ChannelLocation::Service(vec![0]), duty)
    })
    .await
    .expect("el hilo bloqueante deberia terminar")
    .expect("el transporte service deberia levantarse");

    let operation = URL_SAFE.encode("afirma://selectcert?op=selectcert");
    let mut client = a_linger_free_connection(channel.port(), &ca_pem).await;
    client
        .write_all(format!("cmd={operation}idsession={CREDENTIAL}@EOF").as_bytes())
        .await
        .expect("la operacion deberia salir");

    while held.lock().expect("el candado").is_none() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let reply = held
        .lock()
        .expect("el candado")
        .take()
        .expect("el tramite recibio el asa");

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let acknowledgement = reply.answer("CANCEL".to_owned());

    assert!(
        !acknowledgement.wait(Duration::from_millis(300)),
        "el cliente ya se ha ido: el acuse no deberia cumplirse"
    );
}
