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
use rfirma_lib::signing::adapters::tauri::signed_with_the_secret;
use rfirma_lib::signing::application::session::sign_on_token;
use rfirma_lib::signing::ports::{
    ProtectedSecret, SecretPromptError, SecretPromptRequest, SecretPrompter,
};
use rfirma_lib::site::adapters::channel::{bind_first_free, serve, SiteOperations};
use rfirma_lib::site::adapters::data_download::HttpDataSource;
use rfirma_lib::site::adapters::desk::Neighbours;
use rfirma_lib::site::adapters::relay::Relay;
use rfirma_lib::site::adapters::tls::LocalServerCertificate;
use rfirma_lib::site::application::errand::{
    self, Errand, ErrandDesk, ErrandStep, NegotiatedCodec, Transport,
};
use rfirma_lib::site::domain::channel::{ChannelDuty, ChannelLocation, OpenChannel};
use rfirma_lib::site::domain::local_ca::LocalCa;
use rfirma_lib::site::domain::protocol::{
    drawn_ports, read_operation, AfirmaUrl, LaunchRequest, NegotiatedCredential, SafCode,
    SiteOperation, WireAnswer, PROTOCOL_VERSION,
};
use rfirma_lib::site::domain::relay_error::{RelayError, Situation as RelaySituation};
use rfirma_lib::site::ports::{Inbox, ReplyHandle as ErrandReply, Servlets};
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

/// El guion del lote local: `setLocalBatchProcess(true)` con un PDF, un binario y un XML.
const THE_LOCAL_BATCH: &str = "batchlocal";

/// El guion del lote local con el binario declarado `PAdES`, ilegible, y `stoponerror=true`.
const THE_LOCAL_BATCH_WITH_AN_ILLEGIBLE_ITEM: &str = "batchlocalillegible";

/// El guion de `sign` con `format=CAdES` y `mode=explicit` sobre el reto binario.
const THE_SIGN_CADES_EXPLICIT: &str = "signcades";

/// El guion de `sign` con `format=CAdES`, `mode=explicit` y `gzip=true` sobre el reto comprimido.
const THE_SIGN_GZIP: &str = "signgzip";

/// El guion de `sign` con `format=CAdES-ASiC-S` sobre el mismo reto binario.
const THE_SIGN_CADES_ASIC_S: &str = "signcadesasics";

/// El guion de `sign` con `format=auto` sobre el mismo reto binario.
const THE_SIGN_AUTO: &str = "signauto";

/// El guion de `sign` con `format=XAdES` sobre el XML de referencia.
const THE_SIGN_XADES: &str = "signxades";

/// El guion de `sign` con `format=auto` sobre el mismo XML de referencia.
const THE_SIGN_XADES_AUTO: &str = "signxadesauto";

/// El guion de `sign` con `format=PAdES` sobre el PDF que deja la prueba.
const THE_SIGN_PADES: &str = "signpades";

/// El mismo guion de `sign` con `format=PAdES`, con `checkSignatures=true` en las `properties`.
const THE_SIGN_PADES_CHECKING_SIGNATURES: &str = "signpadeschecking";

/// El guion de `sign` con `format=FacturaE` sobre la factura de referencia.
const THE_SIGN_FACTURAE: &str = "signfacturae";

/// El guion de `cosign` con `format=FacturaE` sobre la misma factura.
const THE_COSIGN_FACTURAE: &str = "cosignfacturae";

/// El certificado de pruebas de la FNMT vigente del token `rfirma-test`.
const THE_TEST_CERTIFICATE: &str = "FNMT-ACTIVO-99999999R";

/// El secreto del token de pruebas `rfirma-test`.
const THE_TOKEN_SECRET: &str = "1234";

/// Intentos de atar la ubicación del canal antes de darla por ocupada.
const PORT_ATTEMPTS: usize = 60;

/// Los casos que se turnan: la confianza de la CA local con la que sirven los servlets del lote es
/// del proceso entero.
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
    /// Con `setForceWSMode(true)`: el cliente publicado no abre canal y va por servidor intermedio.
    Relay,
}

impl BenchMode {
    fn as_env_value(self) -> &'static str {
        match self {
            Self::Fourth => "v4",
            Self::Third => "v3",
            Self::Service => "service",
            Self::Relay => "relay",
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
    third_port: Option<u16>,
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
        Self::spawn(material, mode, script, &[], a_free_port())
    }

    /// Arranca el conductor con uno de los guiones que necesitan el PDF que la prueba deja en
    /// disco: el del lote local y los de `sign` con `format=PAdES`.
    fn running_the_script_over_the_pdf(
        material: &ChannelMaterial,
        mode: BenchMode,
        script: &str,
        pdf_path: &Path,
    ) -> Self {
        Self::spawn(
            material,
            mode,
            script,
            &[("RFIRMA_BENCH_PDF", pdf_path.as_os_str())],
            a_free_port(),
        )
    }

    fn spawn(
        material: &ChannelMaterial,
        mode: BenchMode,
        script: &str,
        extra_env: &[(&str, &std::ffi::OsStr)],
        third_port: u16,
    ) -> Self {
        let mut command = Command::new("node");
        command
            .arg(the_driver())
            .env("RFIRMA_AUTOSCRIPT", the_published_client())
            .env("NODE_EXTRA_CA_CERTS", material.ca_pem_file.path())
            .env("RFIRMA_BENCH_TIMEOUT_MS", PATIENCE.as_millis().to_string())
            .env("RFIRMA_BENCH_MODE", mode.as_env_value())
            .env("RFIRMA_BENCH_SCRIPT", script)
            .env("RFIRMA_BENCH_PORT", third_port.to_string())
            .env(
                "RFIRMA_BENCH_SERVLET_CERT",
                material.certificate_pem_file.path(),
            )
            .env("RFIRMA_BENCH_SERVLET_KEY", material.key_pem_file.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let mut child = command
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

        let third_port = matches!(mode, BenchMode::Third).then_some(third_port);
        Self {
            child,
            events,
            third_port,
        }
    }

    /// Dónde abrir el canal: en v3 el puerto que se le dio al conductor, en el resto lo que dijo
    /// la invocación.
    fn the_channel_location(&self, launch: &LaunchRequest) -> ChannelLocation {
        match self.third_port {
            Some(port) => ChannelLocation::Fixed(port),
            None => launch.location().clone(),
        }
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

/// Un puerto de loopback libre ahora mismo, para que cada caso v3 escuche en el suyo.
fn a_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("deberia haber un puerto de loopback libre")
        .port()
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

/// Un fichero temporal con el contenedor ASiC-S ya en disco, que es como lo lee el oráculo.
fn an_asic_s_file(container: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".asics", container)
}

/// Ruta del reto de 64 bytes del banco de referencia, el que firma el guion `sign`.
fn the_challenge_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference/challenge.bin")
}

/// El binario del lote local: nunca empieza por `%PDF-`, así que declararlo `format=PAdES` es lo
/// que lo vuelve ilegible en el guion de `stoponerror`.
const THE_LOCAL_BATCH_BINARY: &[u8] = b"contenido binario del lote local, sin PDF ni XML dentro";

/// Genera un PDF sintético de una página, admisible para `format=PAdES` (ADR-0014).
fn a_one_page_pdf() -> Vec<u8> {
    const PAGE_WIDTH: u32 = 595;
    const PAGE_HEIGHT: u32 = 842;

    let content = "BT /F1 24 Tf 72 750 Td (rfirma: lote local) Tj ET\n".to_owned();
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] \
             /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }

    let xref_at = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

/// Salida esperada de `pdfsig` para una firma válida.
const PDFSIG_VALID: &str = "Signature Validation: Signature is Valid.";

/// Valida la firma PAdES con `pdfsig` (ADR-0014).
fn validated_by_pdfsig(pdf: &Path) {
    let output = Command::new("pdfsig")
        .arg(pdf)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "falta pdfsig: es la puerta de validez de la grada C (ADR-0014).\n  \
                 sudo apt install -y poppler-utils\n{error}"
            )
        });
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "pdfsig ha fallado:\n{report}");
    assert!(
        report.contains(PDFSIG_VALID),
        "pdfsig no da la firma por valida:\n{report}"
    );
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
    SiteOperations::for_operations(|_, _| {})
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
async fn the_third_protocol_connects_to_the_port_rfirma_was_told_to_open() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Third);
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 3 se habla aqui, sin puertos");

    let NegotiatedCredential::Required(credential) = launch.credential() else {
        panic!("el cliente publicado, aunque hable la version 3, sigue mandando idsession");
    };
    assert_eq!(
        credential.as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos, igual que en la version 4"
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        no_operations(),
    )
    .await;
    assert_eq!(
        Some(channel.port()),
        client.third_port,
        "el canal se abre en el puerto que se le dio al cliente publicado"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_message_reveals_the_window_before_the_operation() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(&material);
    let url = client.the_launch_url();
    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("version 4");

    let sequence = Arc::new(Mutex::new(Vec::new()));
    let seq_arr = Arc::clone(&sequence);
    let seq_del = Arc::clone(&sequence);

    let (delivered, waited) = tokio::sync::oneshot::channel();
    let delivered = Arc::new(Mutex::new(Some(delivered)));

    let inbox = SiteOperations::of(
        move || {
            seq_arr.lock().unwrap().push("ventana".to_owned());
        },
        move |_url, reply| {
            seq_del.lock().unwrap().push("operacion".to_owned());
            if let Some(delivered) = delivered.lock().unwrap().take() {
                let _ = delivered.send(());
            }
            reply.answer(r#"{"state":"ok"}"#.to_owned());
        },
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        inbox,
    )
    .await;

    let _ = tokio::time::timeout(Duration::from_secs(10), waited).await;

    let steps = sequence.lock().unwrap().clone();
    assert_eq!(
        steps,
        vec!["ventana".to_owned(), "operacion".to_owned()],
        "el eco o mensaje inicial revela la ventana antes de la operacion"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_message_over_the_third_protocol_reveals_the_window_before_the_operation() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Third);
    let url = client.the_launch_url();
    let parsed = AfirmaUrl::parse(&url).expect("la invocacion deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("version 3");

    let sequence = Arc::new(Mutex::new(Vec::new()));
    let seq_arr = Arc::clone(&sequence);
    let seq_del = Arc::clone(&sequence);

    let (delivered, waited) = tokio::sync::oneshot::channel();
    let delivered = Arc::new(Mutex::new(Some(delivered)));

    let inbox = SiteOperations::of(
        move || {
            seq_arr.lock().unwrap().push("ventana".to_owned());
        },
        move |_url, reply| {
            seq_del.lock().unwrap().push("operacion".to_owned());
            if let Some(delivered) = delivered.lock().unwrap().take() {
                let _ = delivered.send(());
            }
            reply.answer(r#"{"state":"ok"}"#.to_owned());
        },
    );

    let channel = the_channel_at(
        &client.the_channel_location(&launch),
        &material,
        ChannelDuty::Serve(launch.credential().clone()),
        inbox,
    )
    .await;

    let _ = tokio::time::timeout(Duration::from_secs(10), waited).await;

    let steps = sequence.lock().unwrap().clone();
    assert_eq!(
        steps,
        vec!["ventana".to_owned(), "operacion".to_owned()],
        "el primer mensaje en v3 revela la ventana antes de la operacion"
    );

    channel.close();
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
    roots.signing.prompter = Arc::new(MistypesTheTokenSecretOnce);
    roots
}

/// El diálogo del PIN: la primera vez se equivoca y, al avisarle, teclea el del token de pruebas.
struct MistypesTheTokenSecretOnce;

impl SecretPrompter for MistypesTheTokenSecretOnce {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        match request.incorrect_secret {
            false => Ok(ProtectedSecret::from_str("0000")),
            true => Ok(ProtectedSecret::from_str(THE_TOKEN_SECRET)),
        }
    }
}

/// La mesa del trámite montada sobre las raíces de un rFirma en marcha.
fn the_desk_of(roots: &Roots) -> ErrandDesk<'_, Isolate, Isolate, Neighbours<'_>> {
    ErrandDesk {
        engine: &roots.signing.isolate,
        policies: &roots.signing.isolate,
        validation: &roots.signing.isolate,
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

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
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
        &client.the_channel_location(&launch),
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

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
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
            signed_with_the_secret(&desk, live, "")
                .expect("el lote deberia cerrarse con el secreto del token")
        });
    })
}

/// El trámite atendiendo el lote local: consiente con el certificado de pruebas y lo cierra con
/// el secreto del token, firmando cada elemento por el ciclo de sede sin servlets (ADR-0014).
fn the_local_batch_errand_of(
    roots: &Arc<Roots>,
    signer: &Arc<Mutex<Option<Vec<u8>>>>,
) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;

        let answering = ErrandReply::of(move |text| reply.answer(text));
        let Some(ErrandStep::AskingToSignTheLocalBatch(consent)) =
            errand::attend(&desk, url, answering, live)
        else {
            return;
        };
        assert_eq!(
            consent.items.len(),
            3,
            "el lote local del guion lleva tres elementos"
        );

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

        errand::consent(&desk, &chosen.id, live).expect("el lote local deberia quedar consentido");
        tokio::task::block_in_place(|| {
            signed_with_the_secret(&desk, live, "")
                .expect("el lote local deberia cerrarse con el secreto del token")
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

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
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

/// Un elemento del resultado del lote local, por su `id`.
fn local_batch_item<'a>(result: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    result["signs"]
        .as_array()
        .expect("el resultado del lote local trae 'signs'")
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("el lote local no trae el elemento '{id}': {result}"))
}

/// El lote local de tres elementos (PDF/`PAdES`, binario/`CAdES`, XML/`XAdES`) firmado con
/// `setLocalBatchProcess(true)` y sin presigner ni postsigner: las tres firmas validan con la
/// herramienta de su formato.
async fn the_local_batch_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let pdf_file = a_temp_file(".pdf", &a_one_page_pdf());
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        mode,
        THE_LOCAL_BATCH,
        pdf_file.path(),
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_local_batch_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote local tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let result: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(verdict.field("result"))
            .expect("el resultado del lote local llega en base64"),
    )
    .expect("el resultado del lote local es JSON");

    for id in ["pdf", "bin", "xml"] {
        assert_eq!(
            local_batch_item(&result, id)["result"],
            "DONE_AND_SAVED",
            "el elemento '{id}' tenia que firmarse: {result}"
        );
    }

    let pdf_signed = STANDARD
        .decode(
            local_batch_item(&result, "pdf")["signature"]
                .as_str()
                .expect("el PDF firmado llega en base64"),
        )
        .expect("el PDF firmado es base64 valido");
    let pdf_signed_file = a_temp_file(".pdf", &pdf_signed);
    validated_by_pdfsig(pdf_signed_file.path());

    let cms = STANDARD
        .decode(
            local_batch_item(&result, "bin")["signature"]
                .as_str()
                .expect("el CAdES del binario llega en base64"),
        )
        .expect("el CAdES del binario es base64 valido");
    let binary_file = a_temp_file(".bin", THE_LOCAL_BATCH_BINARY);
    verified_by_openssl(&cms, binary_file.path());
    validated_by_the_reference_tool(&cms);

    let xml = STANDARD
        .decode(
            local_batch_item(&result, "xml")["signature"]
                .as_str()
                .expect("el XAdES del XML llega en base64"),
        )
        .expect("el XAdES del XML es base64 valido");
    let xml_file = an_xml_file(&xml);
    well_formed_according_to_xmllint(xml_file.path());
    carries_a_xmldsig_signature(&xml);

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
async fn the_published_client_signs_a_local_batch() {
    the_local_batch_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_local_batch_also_over_the_third_protocol() {
    the_local_batch_of(BenchMode::Third).await;
}

/// El mismo lote local, con el binario declarado `format=PAdES` —y por tanto ilegible, al no ser
/// un PDF— y `stoponerror=true`: el PDF que iba antes se salta también, como el original.
async fn the_local_batch_with_an_illegible_item_of(mode: BenchMode) {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let pdf_file = a_temp_file(".pdf", &a_one_page_pdf());
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        mode,
        THE_LOCAL_BATCH_WITH_AN_ILLEGIBLE_ITEM,
        pdf_file.path(),
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_local_batch_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "success",
        "el lote local tenia que acabar en el successCallback aunque un elemento fallase, y \
         acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );

    let result: serde_json::Value = serde_json::from_slice(
        &STANDARD
            .decode(verdict.field("result"))
            .expect("el resultado del lote local llega en base64"),
    )
    .expect("el resultado del lote local es JSON");

    assert_eq!(
        local_batch_item(&result, "pdf")["result"],
        "SKIPPED",
        "el elemento anterior al que falla tambien se salta: {result}"
    );
    assert_eq!(
        local_batch_item(&result, "bin")["result"],
        "ERROR_PRE",
        "el binario declarado PAdES es ilegible: {result}"
    );
    assert_eq!(
        local_batch_item(&result, "xml")["result"],
        "SKIPPED",
        "el elemento posterior al que falla se salta: {result}"
    );

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_stops_a_local_batch_on_an_illegible_item() {
    the_local_batch_with_an_illegible_item_of(BenchMode::Fourth).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_stops_a_local_batch_on_an_illegible_item_also_over_the_third_protocol(
) {
    the_local_batch_with_an_illegible_item_of(BenchMode::Third).await;
}

/// El trámite atendiendo `sign`: consiente con el certificado de pruebas, firma en el token con
/// el secreto y apunta el DER del firmante para contrastarlo con el que recibe la sede.
fn the_sign_errand_of(roots: &Arc<Roots>, signer: &Arc<Mutex<Option<Vec<u8>>>>) -> SiteOperations {
    let roots = Arc::clone(roots);
    let signer = Arc::clone(signer);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
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

/// El trámite atendiendo una operación que el protocolo rechaza sin pedir consentimiento: la
/// URL se decodifica y la respuesta sale por el canal en el mismo `attend`.
fn the_refusing_errand_of(roots: &Arc<Roots>) -> SiteOperations {
    let roots = Arc::clone(roots);

    SiteOperations::for_operations(move |url, reply: ErrandReply| {
        let desk = the_desk_of(&roots);
        let live = &roots.site.errand;
        let answering = ErrandReply::of(move |text| reply.answer(text));
        errand::attend(&desk, url, answering, live);
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

/// Comprueba `path` con el oráculo de la grada C (`rfirma-native-bridge/testbench/validate.sh`, #526).
fn validated_by_the_reference_tool_at(path: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../rfirma-native-bridge/testbench/validate.sh");
    let output = Command::new(&script)
        .arg(path)
        .output()
        .unwrap_or_else(|error| panic!("no se ha podido ejecutar {}: {error}", script.display()));
    assert!(
        output.status.success(),
        "validate.sh ha fallado:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba el CMS con el oráculo de la grada C (`rfirma-native-bridge/testbench/validate.sh`, #526).
fn validated_by_the_reference_tool(cms: &[u8]) {
    let cms_file = a_der_file(cms);
    validated_by_the_reference_tool_at(cms_file.path());
}

/// Comprueba que el fichero en `path` está bien formado con `xmllint --noout`.
fn well_formed_according_to_xmllint(path: &Path) {
    let output = Command::new("xmllint")
        .arg("--noout")
        .arg(path)
        .output()
        .expect("falta xmllint para el banco de conformidad");
    assert!(
        output.status.success(),
        "xmllint ha rechazado el XML:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Comprueba que el contenedor nombra la firma CAdES que lleva dentro, en vez de dejar que un
/// ZIP cualquiera pase el banco.
fn carries_the_asic_s_binary_signature(container: &[u8]) {
    const ENTRY: &[u8] = b"META-INF/signature.p7s";

    assert!(
        container.windows(ENTRY.len()).any(|window| window == ENTRY),
        "al contenedor le falta {}",
        String::from_utf8_lossy(ENTRY)
    );
}

/// Comprueba que `xml` trae un `ds:Signature` del espacio de nombres XMLDSig, en vez de dejar
/// que un XML simplemente bien formado pase el banco sin firma.
fn carries_a_xmldsig_signature(xml: &[u8]) {
    let xml = String::from_utf8_lossy(xml);
    assert!(
        xml.contains("http://www.w3.org/2000/09/xmldsig#") && xml.contains(":Signature"),
        "el XML no trae un elemento Signature del espacio de nombres XMLDSig:\n{xml}"
    );
}

/// Un `sign()` del cliente publicado del reto binario del banco de referencia, verificado con
/// `openssl cms -verify` y con el oráculo de la grada C. El puente entrega el CAdES detached en
/// ambos guiones, con y sin `mode=explicit`, así que el reto original hace falta en los dos.
async fn the_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

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
async fn the_published_client_signs_a_gzipped_binary_challenge() {
    the_sign_of(BenchMode::Fourth, THE_SIGN_GZIP).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_with_cades_explicit_also_over_the_third_protocol(
) {
    the_sign_of(BenchMode::Third, THE_SIGN_CADES_EXPLICIT).await;
}

/// Dos trámites de sede a la vez en el mismo proceso, cada uno con su propia terna de `ports=`
/// (ID-06): no hay techo de trámites simultáneos ni estado global que los estorbe. No toma
/// `ONE_AT_A_TIME`, porque eso solo lo necesitan los casos de lote con servlets.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn two_published_clients_sign_at_once_in_the_same_process() {
    if !the_bench_can_be_mounted() {
        return;
    }

    tokio::join!(
        the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT),
        the_sign_of(BenchMode::Fourth, THE_SIGN_CADES_EXPLICIT),
    );
}

/// Un `sign()` del cliente publicado con `format=CAdES-ASiC-S`: lo que vuelve no es un CMS sino
/// el contenedor ZIP, con la firma CAdES dentro, y el oráculo de la grada C lo valida.
async fn the_asic_s_sign_of(mode: BenchMode, script: &str) {
    if !the_bench_can_be_mounted() {
        return;
    }

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

    let container = STANDARD
        .decode(verdict.field("result"))
        .expect("el contenedor de sign llega en base64");
    assert_eq!(
        &container[..4],
        b"PK\x03\x04",
        "un ASiC-S es un ZIP y empieza por su firma de fichero local"
    );
    carries_the_asic_s_binary_signature(&container);
    validated_by_the_reference_tool_at(an_asic_s_file(&container).path());

    channel.close();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_a_binary_challenge_into_an_asic_s_container() {
    the_asic_s_sign_of(BenchMode::Fourth, THE_SIGN_CADES_ASIC_S).await;
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
    carries_a_xmldsig_signature(&xml);
    let xml_file = an_xml_file(&xml);
    well_formed_according_to_xmllint(xml_file.path());
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_signs_an_invoice_with_facturae() {
    the_xades_sign_of(BenchMode::Fourth, THE_SIGN_FACTURAE).await;
}

/// Cofirmar una factura no se admite: el `errorCallback` del cliente publicado tiene que
/// recibir `SAF_04` (`ERROR_UNSUPPORTED_OPERATION`), sin que el trámite llegue a pedir
/// consentimiento.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn cosigning_an_invoice_with_facturae_is_refused() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let client =
        PublishedClient::running_the_script(&material, BenchMode::Fourth, THE_COSIGN_FACTURAE);

    let channel =
        the_errand_channel(&client, &material, &roots, the_refusing_errand_of(&roots)).await;

    let verdict = client.next_event();
    assert_eq!(
        verdict.name(),
        "error",
        "la cofirma de una factura tenia que acabar en el errorCallback, y acabo en {}",
        verdict.name()
    );
    assert_eq!(
        verdict.field("message"),
        WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire(),
        "la cofirma de una factura tenia que contestar ERROR_UNSUPPORTED_OPERATION"
    );

    channel.close();
}

/// Una ronda del banco que firma `pdf` con `format=PAdES` y devuelve el PDF firmado, para medir
/// después sobre él la comprobación de las firmas previas.
async fn the_pdf_signed_by_the_bench(pdf: &Path) -> Vec<u8> {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        BenchMode::Fourth,
        THE_SIGN_PADES,
        pdf,
    );

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
        "la primera firma PAdES tenia que acabar en el successCallback, y acabo en {}: {}",
        verdict.name(),
        verdict.field("message")
    );
    let signed = STANDARD
        .decode(verdict.field("result"))
        .expect("el PDF firmado llega en base64");

    channel.close();
    signed
}

/// Lo que el cliente publicado recibe al firmar `pdf` con `checkSignatures=true` en las
/// `properties`.
async fn checking_the_signatures_of(pdf: &Path) -> Event {
    let material = ChannelMaterial::fresh();
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let roots = Arc::new(tokio::task::block_in_place(|| {
        a_running_rfirma(home.path())
    }));
    let signer = Arc::new(Mutex::new(None));
    let client = PublishedClient::running_the_script_over_the_pdf(
        &material,
        BenchMode::Fourth,
        THE_SIGN_PADES_CHECKING_SIGNATURES,
        pdf,
    );

    let channel = the_errand_channel(
        &client,
        &material,
        &roots,
        the_sign_errand_of(&roots, &signer),
    )
    .await;

    let verdict = client.next_event();
    channel.close();
    verdict
}

/// La version del encabezado entra en el `/ByteRange`: el resumen de la firma deja de cuadrar.
fn with_the_signed_bytes_altered(pdf: &[u8]) -> Vec<u8> {
    const HEADER: &[u8] = b"%PDF-1.";

    let at = pdf
        .windows(HEADER.len())
        .position(|window| window == HEADER)
        .expect("el encabezado tiene que estar")
        + HEADER.len();
    let mut altered = pdf.to_vec();
    altered[at] = if altered[at] == b'7' { b'4' } else { b'7' };
    altered
}

/// `checkSignatures=true` medido por el cable entero contra la libreria nativa: sobre el PDF
/// firmado sin tocar el tramite sigue hasta la firma, y sobre el mismo PDF alterado el
/// `errorCallback` del cliente publicado recibe `SAF_39` (`ERROR_INVALID_SIGNATURE`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "grada C: necesita la libreria nativa (RFIRMA_LIB_DIR) y el token de pruebas"]
async fn the_published_client_is_refused_a_document_whose_previous_signature_does_not_hold() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let pdf = a_temp_file(".pdf", &a_one_page_pdf());
    let signed = the_pdf_signed_by_the_bench(pdf.path()).await;

    let signed_file = a_temp_file(".pdf", &signed);
    let held = checking_the_signatures_of(signed_file.path()).await;
    assert_eq!(
        held.name(),
        "success",
        "la firma previa se sostiene, asi que el tramite tenia que seguir hasta firmar, y acabo \
         en {}: {}",
        held.name(),
        held.field("message")
    );

    let altered_file = a_temp_file(".pdf", &with_the_signed_bytes_altered(&signed));
    let broken = checking_the_signatures_of(altered_file.path()).await;
    assert_eq!(
        broken.name(),
        "error",
        "el PDF alterado tenia que acabar en el errorCallback, y acabo en {}",
        broken.name()
    );
    assert_eq!(
        broken.field("message"),
        WireAnswer::refused(SafCode::InvalidSignature).on_the_wire(),
        "una firma previa que ya no cuadra tenia que contestar ERROR_INVALID_SIGNATURE"
    );
}

/// El servidor intermedio del banco visto desde rFirma: sirve lo que el cliente publicado subió y
/// guarda la respuesta, sin mirar la dirección del servlet, que en el banco es siempre el mismo.
#[derive(Default)]
struct BenchServlets {
    files: Mutex<std::collections::HashMap<String, String>>,
}

impl BenchServlets {
    fn put(&self, id: &str, data: &str) {
        self.files
            .lock()
            .expect("el candado")
            .insert(id.to_owned(), data.to_owned());
    }

    fn get(&self, id: &str) -> Option<String> {
        self.files.lock().expect("el candado").get(id).cloned()
    }
}

impl Servlets for BenchServlets {
    fn retrieve(&self, _service_url: &str, id: &str) -> Result<String, RelayError> {
        self.get(id).ok_or_else(|| {
            RelayError::new(
                RelaySituation::ServletUnreachable,
                format!("el banco no tiene guardado {id}"),
            )
        })
    }

    fn store(&self, _service_url: &str, id: &str, data: &str) -> Result<(), RelayError> {
        self.put(id, data);
        Ok(())
    }

    fn wait(&self, _service_url: &str, _id: &str) -> Result<(), RelayError> {
        Ok(())
    }
}

/// El documento que el guion `relay` firma, el mismo que arma el conductor.
fn the_document_too_long_for_the_url() -> Vec<u8> {
    let mut document = b"%PDF-1.7\n".to_vec();
    document.extend(std::iter::repeat_n(b'd', 3000));
    document
}

/// La invocación y lo que el cliente publicado subió al servlet, en el orden en el que los emite.
fn the_relay_launch_of(client: &PublishedClient, servlets: &BenchServlets) -> String {
    let mut launch = None;
    for _ in 0..2 {
        let event = client.next_event();
        match event.name() {
            "stored" => servlets.put(event.field("id"), event.field("dat")),
            "launch" => launch = Some(event.field("url").to_owned()),
            _ => panic!("el banco no esperaba este evento: {}", event.0),
        }
    }
    launch.expect("el cliente publicado tiene que lanzar la aplicacion")
}

#[test]
fn the_published_client_forced_to_the_relay_launches_without_stservlet_and_rfirma_reads_it() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_as(&material, BenchMode::Relay);
    let servlets = Arc::new(BenchServlets::default());

    let launch = the_relay_launch_of(&client, &servlets);
    assert!(
        launch.contains("fileid=") && launch.contains("rtservlet=") && launch.contains("key="),
        "el preproceso de URL larga lanza con 'fileid', 'rtservlet' y 'key': {launch}"
    );
    assert!(
        !launch.contains("stservlet=") && !launch.contains("&id="),
        "la sede en modo servidor intermedio no manda 'stservlet' ni 'id' en la URL: {launch}"
    );

    let request = LaunchRequest::parse(&launch).expect("rfirma deberia leer la invocacion");
    let location = request.location().clone();

    let delivered: Arc<Mutex<Option<(AfirmaUrl, ErrandReply)>>> = Arc::new(Mutex::new(None));
    let inbox_delivered = Arc::clone(&delivered);
    let inbox = Inbox::for_operations(move |url, reply| {
        *inbox_delivered.lock().expect("el candado") = Some((url, reply));
    });
    let relay = Relay::new(
        Arc::clone(&servlets) as Arc<dyn Servlets + Send + Sync>,
        inbox,
        Arc::new(|_| {}),
    );

    let mut channel = relay
        .open(&location, ChannelDuty::Serve(NegotiatedCredential::Absent))
        .expect("el arranque por servidor intermedio deberia abrirse");
    channel
        .take_delivery()
        .expect("una operacion Serve siempre trae entrega")
        .now();

    let (operation, reply) = delivered
        .lock()
        .expect("el candado")
        .take()
        .expect("la operacion deberia haberse entregado");
    let stored_at = operation
        .parameter("id")
        .expect("el XML de parametros trae el 'id'")
        .to_owned();
    let SiteOperation::Sign(signing) =
        read_operation(&operation, &HttpDataSource).expect("lee la operacion")
    else {
        panic!("el guion del banco pide una firma");
    };
    assert_eq!(signing.document(), the_document_too_long_for_the_url());

    reply.answer("la-respuesta-del-tramite".to_owned());
    assert_eq!(
        servlets.get(&stored_at).as_deref(),
        Some("la-respuesta-del-tramite"),
        "la respuesta sube con el 'id' que venia dentro del XML de parametros"
    );
}
