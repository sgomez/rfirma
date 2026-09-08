//! Banco de conformidad contra autoscript.js oficial ejecutado en Node (ADR-0014).

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use rfirma_lib::desktop::adapters::paths::Paths;
use rfirma_lib::identity::domain::store::Store;
use rfirma_lib::signing::adapters::isolate::Isolate;
use rfirma_lib::signing::application::session::sign_on_token;
use rfirma_lib::site::adapters::channel::{bind_first_free, serve, SiteOperations};
use rfirma_lib::site::adapters::desk::Neighbours;
use rfirma_lib::site::adapters::tls::LocalServerCertificate;
use rfirma_lib::site::application::errand::{
    self, Errand, ErrandDesk, ErrandStep, NegotiatedCodec,
};
use rfirma_lib::site::domain::channel::{ChannelDuty, ChannelLocation, OpenChannel};
use rfirma_lib::site::domain::local_ca::LocalCa;
use rfirma_lib::site::domain::protocol::{
    drawn_ports, AfirmaUrl, LaunchRequest, NegotiatedCredential, SafCode, WireAnswer,
    PROTOCOL_VERSION, THE_PORT_OF_THE_THIRD_PROTOCOL,
};
use rfirma_lib::site::ports::ReplyHandle as ErrandReply;
use rfirma_lib::Roots;

/// La versión de `service` que habla el cliente publicado cuando no hay WebSocket.
const THE_SERVICE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS: i64 = 1;

/// Tiempo máximo de espera para respuestas en pruebas.
const PATIENCE: Duration = Duration::from_secs(40);

/// La versión que el cliente publicado habla por defecto, y la que rfirma implementa.
const THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS: i64 = 4;

/// El guion de una sola selección, el de los casos que solo miran la invocación de arranque.
const THE_SINGLE_SELECTION: &str = "selectcert";

/// El guion de tres selecciones con el certificado fijado y soltado.
const THE_STICKY_SELECTIONS: &str = "sticky";

/// El guion del lote remoto: dos documentos firmados con `signBatchJSON`.
const THE_REMOTE_BATCH: &str = "batch";

/// El guion del lote remoto heredado: dos documentos firmados con `signBatch` en XML.
const THE_LEGACY_XML_BATCH: &str = "batchxml";

/// El guion del lote remoto sin presigner escuchando.
const THE_REMOTE_BATCH_WITH_THE_DOWN_PRESIGNER: &str = "batchdown";

/// El guion de `sign` con `format=CAdES` y `mode=explicit` sobre el reto binario.
const THE_SIGN_CADES_EXPLICIT: &str = "signcades";

/// El guion de `sign` con `format=auto` sobre el mismo reto binario.
const THE_SIGN_AUTO: &str = "signauto";

/// El guion de `sign` con `format=XAdES` sobre el XML de referencia.
const THE_SIGN_XADES: &str = "signxades";

/// El guion de `sign` con `format=auto` sobre el mismo XML de referencia.
const THE_SIGN_XADES_AUTO: &str = "signxadesauto";

/// El certificado de pruebas de la FNMT vigente del token `rfirma-test`.
const THE_TEST_CERTIFICATE: &str = "FNMT-ACTIVO-99999999R";

/// El secreto del token de pruebas `rfirma-test`.
const THE_TOKEN_SECRET: &str = "1234";

/// Intentos de atar la ubicación del canal antes de darla por ocupada.
const PORT_ATTEMPTS: usize = 60;

/// Los casos que se turnan: el puerto fijo del protocolo 3 es el mismo en todos, y la confianza de
/// la CA local con la que sirven los servlets es del proceso entero.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Modo en el que se fuerza al `autoscript.js` publicado a hablar, porque nunca manda `v=3` por
/// websocket por su cuenta.
#[derive(Clone, Copy)]
enum BenchMode {
    /// El que el cliente publicado habla de por sí: puertos sorteados y `v=4`.
    Fourth,
    /// El fuente reescrito antes de ejecutarlo: sin `ports`, `v=3` y el puerto fijo.
    Third,
    /// Sin `WebSocket` en el entorno: el cliente publicado cae a `afirma://service?v=1`.
    Service,
}

impl BenchMode {
    fn as_env_value(self) -> &'static str {
        match self {
            Self::Fourth => "v4",
            Self::Third => "v3",
            Self::Service => "service",
        }
    }
}

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/driver.mjs")
}

/// Comprueba disponibilidad de Node y del script de autoscript.js.
fn the_bench_can_be_mounted() -> bool {
    let missing = if !the_published_client().exists() {
        Some(format!(
            "falta {}: ejecuta `just autoscript`",
            the_published_client().display()
        ))
    } else if Command::new("node").arg("--version").output().is_err() {
        Some("falta Node en el PATH".to_owned())
    } else {
        None
    };

    match missing {
        None => true,
        Some(reason) => {
            assert!(
                std::env::var_os("CI").is_none(),
                "el banco de conformidad no es opcional en el CI: {reason}"
            );
            eprintln!("banco de conformidad saltado ({reason}); en el CI esto es un fallo");
            false
        }
    }
}

/// Un evento del conductor: una línea de JSON de su salida estándar.
struct Event(String);

impl Event {
    fn name(&self) -> &str {
        self.field("event")
    }

    /// Extrae el valor de un campo del objeto JSON del evento.
    fn field(&self, name: &str) -> &str {
        let needle = format!("\"{name}\":\"");
        let Some(from) = self.0.find(&needle) else {
            return "";
        };
        let rest = &self.0[from + needle.len()..];
        rest.split('"').next().unwrap_or("")
    }
}

/// El cliente publicado corriendo bajo Node, con su salida ya en cola.
struct PublishedClient {
    child: Child,
    events: Receiver<Event>,
}

impl PublishedClient {
    /// Arranca el conductor con la CA local en NODE_EXTRA_CA_CERTS, en el modo por defecto (v4).
    fn running_against(material: &ChannelMaterial) -> Self {
        Self::running_as(material, BenchMode::Fourth)
    }

    /// Arranca el conductor con la CA local en NODE_EXTRA_CA_CERTS, en el modo indicado.
    fn running_as(material: &ChannelMaterial, mode: BenchMode) -> Self {
        Self::running_the_script(material, mode, THE_SINGLE_SELECTION)
    }

    /// Arranca el conductor con uno de los guiones del banco, y con el material con el que sus
    /// servlets sirven TLS.
    fn running_the_script(material: &ChannelMaterial, mode: BenchMode, script: &str) -> Self {
        let mut child = Command::new("node")
            .arg(the_driver())
            .env("RFIRMA_AUTOSCRIPT", the_published_client())
            .env("NODE_EXTRA_CA_CERTS", material.ca_pem_file.path())
            .env("RFIRMA_BENCH_TIMEOUT_MS", PATIENCE.as_millis().to_string())
            .env("RFIRMA_BENCH_MODE", mode.as_env_value())
            .env("RFIRMA_BENCH_SCRIPT", script)
            .env(
                "RFIRMA_BENCH_SERVLET_CERT",
                material.certificate_pem_file.path(),
            )
            .env("RFIRMA_BENCH_SERVLET_KEY", material.key_pem_file.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Node deberia arrancar el conductor del banco");

        let stdout = child.stdout.take().expect("el conductor escribe eventos");
        let (sender, events) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if sender.send(Event(line)).is_err() {
                    break;
                }
            }
        });

        Self { child, events }
    }

    /// Siguiente evento emitido por el cliente publicado.
    fn next_event(&self) -> Event {
        match self.events.recv_timeout(PATIENCE) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => {
                panic!("el cliente publicado no dijo nada en {PATIENCE:?}")
            }
            Err(RecvTimeoutError::Disconnected) => {
                panic!("el conductor murio sin dar un veredicto")
            }
        }
    }

    /// URL afirma:// construida por el cliente publicado.
    fn the_launch_url(&self) -> String {
        let event = self.next_event();
        assert_eq!(
            event.name(),
            "launch",
            "el primer evento del banco es la invocacion, y llego {}",
            event.0
        );
        event.field("url").to_owned()
    }
}

impl Drop for PublishedClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Material criptográfico temporal para el canal y para los dos servlets del lote.
struct ChannelMaterial {
    certificate: LocalServerCertificate,
    ca_pem_file: tempfile::NamedTempFile,
    certificate_pem_file: tempfile::NamedTempFile,
    key_pem_file: tempfile::NamedTempFile,
}

impl ChannelMaterial {
    fn fresh() -> Self {
        let ca = LocalCa::generate().expect("la CA local deberia generarse");
        let certificate =
            LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");

        Self {
            ca_pem_file: a_pem_file(&ca.certificate_pem().expect("la CA local en PEM")),
            certificate_pem_file: a_pem_file(
                &certificate
                    .certificate_pem()
                    .expect("el certificado del servidor local en PEM"),
            ),
            key_pem_file: a_pem_file(
                &certificate
                    .private_key_pem()
                    .expect("la clave del servidor local en PEM"),
            ),
            certificate,
        }
    }
}

/// Un fichero temporal con `bytes` ya en disco, con el `suffix` indicado.
fn a_temp_file(suffix: &str, bytes: &[u8]) -> tempfile::NamedTempFile {
    use std::io::Write;

    let mut file = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("un fichero temporal");
    file.write_all(bytes)
        .expect("el fichero deberia escribirse");
    file.flush().expect("el fichero deberia quedar en disco");
    file
}

/// Un fichero temporal con el PEM ya en disco, que es como lo leen Node y OpenSSL.
fn a_pem_file(pem: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".pem", pem)
}

/// Un fichero temporal con el DER ya en disco, que es como lo lee OpenSSL.
fn a_der_file(der: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".der", der)
}

/// Un fichero temporal con el XML ya en disco, que es como lo leen `xmllint` y el oráculo.
fn an_xml_file(xml: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".xml", xml)
}

/// Ruta del reto de 64 bytes del banco de referencia, el que firma el guion `sign`.
fn the_challenge_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference/challenge.bin")
}

/// Abre el canal en uno de los puertos sorteados por la URL.
async fn the_channel_on_one_of(
    url: &AfirmaUrl,
    material: &ChannelMaterial,
    duty: ChannelDuty,
) -> OpenChannel {
    the_channel_at(
        &ChannelLocation::Drawn(drawn_ports(url)),
        material,
        duty,
        no_operations(),
    )
    .await
}

/// Canal que no atiende ninguna operación: el caso se acaba antes de que llegue.
fn no_operations() -> SiteOperations {
    Arc::new(|_, _| {})
}

/// Abre el canal en la ubicación indicada: uno de los puertos sorteados, o el puerto fijo del
/// protocolo 3.
async fn the_channel_at(
    location: &ChannelLocation,
    material: &ChannelMaterial,
    duty: ChannelDuty,
    operations: SiteOperations,
) -> OpenChannel {
    serve(
        bound_once_free(location).await,
        &material.certificate,
        duty,
        operations,
    )
    .await
    .expect("el canal deberia levantarse")
}

/// Ata la ubicación esperando a que se libere: el puerto fijo del protocolo 3 es el mismo en cada
/// invocación del guion, y la anterior tarda en soltarlo.
async fn bound_once_free(location: &ChannelLocation) -> TcpListener {
    let mut refusal = String::new();
    for _ in 0..PORT_ATTEMPTS {
        match bind_first_free(location) {
            Ok(listener) => return listener,
            Err(error) => {
                refusal = error.to_string();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
    panic!("la ubicacion del canal no se libero: {refusal}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_url_the_published_client_builds_is_the_one_rfirma_reads() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 4 se habla aqui");

    assert_eq!(
        PROTOCOL_VERSION, THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS,
        "rfirma implementa la version que el cliente publicado envia"
    );
    let ChannelLocation::Drawn(ports) = launch.location() else {
        panic!(
            "el cliente publicado sortea puertos: {:?}",
            launch.location()
        );
    };
    assert_eq!(
        ports.len(),
        3,
        "el cliente publicado sortea tres puertos, y llegaron {ports:?}"
    );
    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado trae credencial de canal");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unsupported_version_reaches_the_error_callback_of_the_published_client() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();

    let unsupported = url.replace(
        &format!("&v={THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS}"),
        "&v=99",
    );
    let refusal = LaunchRequest::parse(&unsupported).expect_err("la version 99 no se habla aqui");
    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);

    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let _channel =
        the_channel_on_one_of(&parsed, &material, ChannelDuty::Refuse(refusal.answer())).await;

    let verdict = client.next_event();

    assert_eq!(
        verdict.name(),
        "error",
        "el trámite tiene que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("type"),
        "java.lang.InterruptedException",
        "lo medido contra el tag v1.9.2: el cierre del canal es lo que el \
         cliente publicado convierte en error, no el `SAF_21` que le contestamos"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_third_protocol_forces_the_published_client_onto_the_fixed_port() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Third);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 3 se habla aqui, sin puertos");

    assert_eq!(
        launch.location(),
        &ChannelLocation::Fixed(THE_PORT_OF_THE_THIRD_PROTOCOL),
        "el modo v3 fuerza el fuente para que no mande 'ports' y hable la version 3"
    );
    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado, aunque hable la version 3, sigue mandando idsession");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos, igual que en la version 4"
    );

    let channel = the_channel_at(
        launch.location(),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        no_operations(),
    )
    .await;
    assert_eq!(
        channel.port(),
        THE_PORT_OF_THE_THIRD_PROTOCOL,
        "el canal se abre en el puerto fijo, no en uno sorteado"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_websocket_the_published_client_falls_back_to_service_v1() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Service);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 1 de 'service' se habla aqui");

    assert_eq!(
        launch.version(),
        THE_SERVICE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS,
        "sin WebSocket el cliente publicado habla la version 1 de 'service'"
    );
    let ChannelLocation::Service(ports) = launch.location() else {
        panic!(
            "el cliente publicado sortea puertos tambien para 'service': {:?}",
            launch.location()
        );
    };
    assert!(
        !ports.is_empty(),
        "'service' sortea puertos igual que la version 4 de 'websocket'"
    );
    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado manda idsession tambien en 'service'");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos, igual que en 'websocket'"
    );
}

/// El módulo PKCS#11 del token de pruebas.
fn the_test_module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. La grada C necesita SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just token",
        module.display()
    );
    module
}

/// Las cinco raíces de un rFirma en marcha que solo ve el token de pruebas y recuerda bajo esa
/// carpeta, para que el caso arranque siempre sin certificado recordado.
fn a_running_rfirma(home: &std::path::Path) -> Roots {
    let mut roots = rfirma_lib::roots(Paths::under(home));
    roots.identity.stores = vec![Store::module(the_test_module())];
    roots
}

/// La mesa del trámite montada sobre las raíces de un rFirma en marcha.
fn the_desk_of(roots: &Roots) -> ErrandDesk<'_, Isolate, Isolate, Neighbours<'_>> {
    ErrandDesk {
        engine: &roots.signing.isolate,
        policies: &roots.signing.isolate,
        neighbours: Neighbours {
            identity: &roots.identity,
            documents: &roots.documents,
            signing: &roots.signing,
        },
        scratch_dir: roots.site.scratch_dir.clone(),
        scratch: roots.site.scratch.clone(),
        batch: roots.site.batch.clone(),
    }
}

/// El trámite atendiendo la operación del canal, consintiendo con el certificado de pruebas cuando
/// se lo pide, y llevando la cuenta de las veces que lo ha pedido.
fn the_errand_of(roots: &Arc<Roots>, consents: &Arc<AtomicUsize>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let consents = Arc::clone(consents);

    Arc::new(move |url, reply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingForConsent { certificates, .. }) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        consents.fetch_add(1, Ordering::SeqCst);
        let chosen = certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just token`")
            });
        errand::consent(&desk, &chosen.id, live).expect("el consentimiento deberia entregarse");
    })
}

/// El códec con el que rFirma contesta a una invocación de esa versión.
fn the_codec_of(roots: &Roots, launch: &LaunchRequest) -> NegotiatedCodec {
    match launch.version() {
        THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS => Arc::clone(&roots.site.codecs.v4),
        3 => Arc::clone(&roots.site.codecs.v3),
        other => panic!("el guion no habla la version {other}"),
    }
}

/// Abre el canal donde la sede invocó y arranca el trámite que atenderá su operación.
async fn the_errand_channel(
    client: &PublishedClient,
    material: &ChannelMaterial,
    roots: &Arc<Roots>,
    operations: SiteOperations,
) -> OpenChannel {
    let url = client.the_launch_url();
    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la invocacion deberia atenderse");

    let channel = the_channel_at(
        launch.location(),
        material,
        ChannelDuty::Serve(launch.credential().clone()),
        operations,
    )
    .await;
    assert!(
        roots.site.errand.begin(Errand::of(
            launch.credential().clone(),
            channel.port(),
            the_codec_of(roots, &launch),
        )),
        "la invocacion anterior deberia haber cerrado su tramite"
    );
    channel
}

/// Atiende la siguiente selección del guion: abre el canal donde la sede lo invocó, deja que el
/// trámite la conteste y devuelve el evento del `successCallback` del cliente publicado.
async fn the_next_selection(
    client: &PublishedClient,
    material: &ChannelMaterial,
    roots: &Arc<Roots>,
    consents: &Arc<AtomicUsize>,
) -> Event {
    let channel = the_errand_channel(client, material, roots, the_errand_of(roots, consents)).await;
    let event = client.next_event();
    channel.close();
    event
}

/// El certificado que el `successCallback` del cliente publicado recibió.
fn the_certificate_of(event: &Event, step: &str) -> String {
    assert_eq!(
        event.name(),
        "success",
        "la seleccion '{step}' tenia que acabar en el successCallback, y acabo en {}: {}",
        event.name(),
        event.field("message")
    );
    assert_eq!(event.field("step"), step, "las selecciones llegan en orden");
    event.field("data").to_owned()
}

/// Tres selecciones seguidas del cliente publicado: `sticky` contesta la segunda sin volver a
/// preguntar, y `resetsticky` hace que la tercera se vuelva a preguntar.
async fn the_sticky_selections_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let consents = Arc::new(AtomicUsize::new(0));
    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_the_script(&material, mode, THE_STICKY_SELECTIONS);

    let stuck = the_next_selection(&client, &material, &roots, &consents).await;
    let first = the_certificate_of(&stuck, "stuck");
    assert_eq!(
        consents.load(Ordering::SeqCst),
        1,
        "la primera seleccion siempre pregunta"
    );

    let again = the_next_selection(&client, &material, &roots, &consents).await;
    assert_eq!(
        the_certificate_of(&again, "stuck-again"),
        first,
        "sticky devuelve el mismo certificado que quedo fijado"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        1,
        "con sticky la segunda seleccion se contesta sin momento de consentimiento"
    );

    let released = the_next_selection(&client, &material, &roots, &consents).await;
    assert_eq!(
        the_certificate_of(&released, "released"),
        first,
        "tras resetsticky se vuelve a entregar el certificado, ya consentido de nuevo"
    );
    assert_eq!(
        consents.load(Ordering::SeqCst),
        2,
        "resetsticky olvida el fijado y la seleccion vuelve a preguntar"
    );

    let done = client.next_event();
    assert_eq!(done.name(), "done", "el guion tenia que acabar entero");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_spares_the_second_selection_of_the_published_client_from_asking_again() {
    the_sticky_selections_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn sticky_spares_the_second_selection_also_over_the_third_protocol() {
    the_sticky_selections_of(BenchMode::Third).await;
}

/// El fichero congelado con el que contesta uno de los servlets del lote, sin espacios: la sangría
/// la pone el formateador del repositorio y el cliente publicado lo reserializa compacto.
fn the_frozen(fixture: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/conformance")
        .join(fixture);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("falta la fixture {}: {error}", path.display()));
    without_spaces(&text)
}

fn without_spaces(text: &str) -> String {
    text.split_whitespace().collect()
}

/// La CA local, en el fichero que OpenSSL lee como almacén de confianza: el cliente de los dos
/// servlets es el de producción y valida TLS con el almacén del sistema, así que sin esto un
/// servlet del banco seria un «servlet inalcanzable».
fn the_local_ca_trusted_by_the_batch_client(ca_pem_path: &std::path::Path) {
    std::env::set_var("SSL_CERT_FILE", ca_pem_path);
}

/// El trámite atendiendo el lote remoto: consiente con el certificado de pruebas, lo cierra con el
/// secreto del token y apunta el DER del firmante para contrastarlo con el que recibe la sede.
fn the_batch_errand_of(roots: &Arc<Roots>, signer: &Arc<Mutex<Option<Vec<u8>>>>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    Arc::new(move |url, reply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSignTheBatch(consent)) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };
        assert_eq!(consent.signs, 2, "el lote del guion lleva dos documentos");

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just token`")
            });
        let signing_certificate = roots
            .identity
            .chosen(&chosen.id)
            .expect("el certificado consentido deberia seguir en el token");
        *signer
            .lock()
            .expect("nadie envenena el apunte del firmante") =
            Some(signing_certificate.der().to_vec());

        errand::consent(&desk, &chosen.id, live).expect("el lote deberia quedar consentido");
        tokio::task::block_in_place(|| {
            errand::finish_the_batch(&desk, THE_TOKEN_SECRET, live)
                .expect("el lote deberia cerrarse con el secreto del token")
        });
    })
}

/// El lote remoto de dos documentos, del `signBatchJSON` del cliente publicado al resultado
/// congelado del postsigner, pasando por los dos servlets que levanta el conductor.
async fn the_remote_batch_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, THE_REMOTE_BATCH);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_batch_errand_of(&roots, &signer),
    )
    .await;

    let presign = client.next_event();
    assert_eq!(
        presign.name(),
        "presign",
        "el presigner tenia que recibir el lote antes que nada, y llego {}",
        presign.0
    );
    assert_eq!(presign.field("signs"), "2", "el lote lleva dos documentos");
    assert_eq!(
        presign.field("certs"),
        "1",
        "el lote viaja con la cadena del unico firmante"
    );
    assert_eq!(
        presign.field("algorithm"),
        "SHA256",
        "el algoritmo que declara el lote es el que llega al servlet"
    );

    let postsign = client.next_event();
    assert_eq!(
        postsign.name(),
        "postsign",
        "el postsigner tenia que recibir el tridata firmado, y llego {}",
        postsign.0
    );
    assert_eq!(
        postsign.field("signs"),
        "2",
        "las dos firmas del lote llegan con su PK1"
    );
    assert_eq!(
        postsign.field("pre"),
        "1",
        "solo la firma con NEED_PRE=true conserva su PRE"
    );

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let result = STANDARD
        .decode(verdict.field("result"))
        .expect("el resultado del lote llega en base64");
    assert_eq!(
        without_spaces(&String::from_utf8(result).expect("el resultado del lote es texto")),
        the_frozen("batch-postsign-result.json"),
        "el cliente publicado recibe el resultado del postsigner tal cual"
    );
    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "con needcert el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_json() {
    the_remote_batch_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_also_over_the_third_protocol() {
    the_remote_batch_of(BenchMode::Third).await;
}

/// El lote remoto heredado en XML, del `signBatch` del cliente publicado al resultado congelado
/// del postsigner, pasando por los dos servlets que levanta el conductor.
async fn the_remote_xml_batch_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, THE_LEGACY_XML_BATCH);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_batch_errand_of(&roots, &signer),
    )
    .await;

    let presign = client.next_event();
    assert_eq!(
        presign.name(),
        "presign",
        "el presigner tenia que recibir el lote antes que nada, y llego {}",
        presign.0
    );
    assert_eq!(presign.field("signs"), "2", "el lote lleva dos documentos");
    assert_eq!(
        presign.field("certs"),
        "1",
        "el lote viaja con la cadena del unico firmante"
    );
    assert_eq!(
        presign.field("algorithm"),
        "SHA256",
        "el algoritmo que declara el lote es el que llega al servlet"
    );

    let postsign = client.next_event();
    assert_eq!(
        postsign.name(),
        "postsign",
        "el postsigner tenia que recibir el tridata firmado, y llego {}",
        postsign.0
    );
    assert_eq!(
        postsign.field("signs"),
        "2",
        "las dos firmas del lote llegan con su PK1"
    );
    assert_eq!(
        postsign.field("pre"),
        "1",
        "solo la firma con NEED_PRE=true conserva su PRE"
    );

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let result = STANDARD
        .decode(verdict.field("result"))
        .expect("el resultado del lote llega en base64");
    assert_eq!(
        without_spaces(&String::from_utf8(result).expect("el resultado del lote es texto")),
        the_frozen("batch-xml-postsign-result.xml"),
        "el cliente publicado recibe el resultado del postsigner tal cual"
    );
    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "con needcert el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_legacy_xml() {
    the_remote_xml_batch_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_remote_batch_in_legacy_xml_also_over_the_third_protocol() {
    the_remote_xml_batch_of(BenchMode::Third).await;
}

/// El trámite atendiendo el lote remoto cuyo presigner esta caido: consiente y cierra con el
/// secreto del token, y comprueba que el lote se rechaza antes de llegar al postsigner.
fn the_down_presigner_batch_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    Arc::new(move |url, reply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSignTheBatch(consent)) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just token`")
            });

        errand::consent(&desk, &chosen.id, live).expect("el lote deberia quedar consentido");
        tokio::task::block_in_place(|| {
            let outcome = errand::finish_the_batch(&desk, THE_TOKEN_SECRET, live);
            assert!(
                outcome.is_err(),
                "el lote deberia rechazarse con el presigner caido"
            );
        });
    })
}

/// Sin presigner escuchando, el `errorCallback` del cliente publicado tiene que recibir `SAF_26`
/// (`ERROR_CONTACT_BATCH_SERVICE`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_remote_batch_fails_when_the_presigner_is_down() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    the_local_ca_trusted_by_the_batch_client(material.ca_pem_file.path());

    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let client = PublishedClient::running_the_script(
        &material,
        BenchMode::Fourth,
        THE_REMOTE_BATCH_WITH_THE_DOWN_PRESIGNER,
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_down_presigner_batch_errand_of(&roots),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "el lote tenia que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(SafCode::ContactBatchService).on_the_wire(),
        "el presigner caido tenia que contestar ERROR_CONTACT_BATCH_SERVICE"
    );

    channel.close();
}

/// El trámite atendiendo `sign`: consiente con el certificado de pruebas, firma en el token con
/// el secreto y apunta el DER del firmante para contrastarlo con el que recibe la sede.
fn the_sign_errand_of(roots: &Arc<Roots>, signer: &Arc<Mutex<Option<Vec<u8>>>>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    Arc::new(move |url, reply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSign(consent)) = errand::attend(&desk, url, answering, live)
        else {
            return;
        };

        let chosen = consent
            .certificates
            .iter()
            .find(|row| row.label == THE_TEST_CERTIFICATE && row.status.is_usable())
            .unwrap_or_else(|| {
                panic!("el token de pruebas no ofrecio {THE_TEST_CERTIFICATE}: monta `just token`")
            });
        let signing_certificate = roots
            .identity
            .chosen(&chosen.id)
            .expect("el certificado consentido deberia seguir en el token");
        *signer
            .lock()
            .expect("nadie envenena el apunte del firmante") =
            Some(signing_certificate.der().to_vec());

        errand::consent(&desk, &chosen.id, live).expect("la prefirma deberia consentirse");
        tokio::task::block_in_place(|| {
            sign_on_token(
                &roots.identity.signer(),
                &roots.signing.session,
                THE_TOKEN_SECRET,
            )
            .expect("la firma en el token deberia completarse");
            errand::finish(&desk, live).expect("la postfirma deberia completarse");
        });
    })
}

/// Comprueba el CMS detached con `openssl cms -verify`, contra el `content` que firmó.
fn verified_by_openssl(cms: &[u8], content: &Path) {
    let cms_file = a_der_file(cms);
    let output = Command::new("openssl")
        .args(["cms", "-verify", "-noverify", "-inform", "DER", "-in"])
        .arg(cms_file.path())
        .arg("-binary")
        .arg("-content")
        .arg(content)
        .args(["-out", "/dev/null"])
        .output()
        .expect("falta openssl para el banco de conformidad");
    assert!(
        output.status.success(),
        "openssl cms -verify ha fallado:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba `path` con el oráculo de la grada C (`just validate-signature`, #526).
fn validated_by_the_reference_tool_at(path: &Path) {
    let output = Command::new("just")
        .arg("validate-signature")
        .arg(path)
        .output()
        .expect("falta just para el banco de conformidad");
    assert!(
        output.status.success(),
        "just validate-signature ha fallado:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba el CMS con el oráculo de la grada C (`just validate-signature`, #526).
fn validated_by_the_reference_tool(cms: &[u8]) {
    let cms_file = a_der_file(cms);
    validated_by_the_reference_tool_at(cms_file.path());
}

/// Comprueba que `xml` está bien formado con `xmllint --noout`.
fn well_formed_according_to_xmllint(xml: &[u8]) {
    let xml_file = an_xml_file(xml);
    let output = Command::new("xmllint")
        .args(["--noout"])
        .arg(xml_file.path())
        .output()
        .expect("falta xmllint para el banco de conformidad");
    assert!(
        output.status.success(),
        "xmllint ha rechazado el XML:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Un `sign()` del cliente publicado del reto binario del banco de referencia, verificado con
/// `openssl cms -verify` y con el oráculo de la grada C. El puente entrega el CAdES detached en
/// ambos guiones, con y sin `mode=explicit`, así que el reto original hace falta en los dos.
async fn the_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, script);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "'{script}' tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let cms = STANDARD
        .decode(verdict.field("result"))
        .expect("el CMS de sign llega en base64");
    verified_by_openssl(&cms, &the_challenge_path());
    validated_by_the_reference_tool(&cms);

    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_cades_explicit() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_cades_explicit_also_over_the_third_protocol(
) {
    the_sign_of(BenchMode::Third, THE_SIGN_CADES_EXPLICIT).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_format_auto() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_AUTO).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_format_auto_also_over_the_third_protocol(
) {
    the_sign_of(BenchMode::Third, THE_SIGN_AUTO).await;
}

/// Un `sign()` del cliente publicado del XML de referencia, con `ds:Signature` bien formado
/// (`xmllint`) y aceptado por el oráculo de la grada C. `format=XAdES` y `format=auto` sobre XML
/// resuelven la misma variante Enveloping, así que comparten guion de verificación.
async fn the_xades_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let _turn = ONE_AT_A_TIME.lock().await;
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script(&material, mode, script);

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "'{script}' tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let xml = STANDARD
        .decode(verdict.field("result"))
        .expect("el XML de sign llega en base64");
    well_formed_according_to_xmllint(&xml);
    let xml_file = an_xml_file(&xml);
    validated_by_the_reference_tool_at(xml_file.path());

    assert_eq!(
        verdict.field("certificate"),
        STANDARD.encode(
            signer
                .lock()
                .expect("nadie envenena el apunte del firmante")
                .as_ref()
                .expect("el tramite tenia que haber consentido con un certificado")
        ),
        "el successCallback recibe tambien el DER del firmante"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_xades() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_XADES).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_xades_also_over_the_third_protocol() {
    the_xades_sign_of(BenchMode::Third, THE_SIGN_XADES).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_format_auto() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_XADES_AUTO).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_xml_document_with_format_auto_also_over_the_third_protocol()
{
    the_xades_sign_of(BenchMode::Third, THE_SIGN_XADES_AUTO).await;
}
