//! Andamiaje compartido del banco de conformidad (ADR-0014): el cliente publicado bajo Node y el canal que abre rFirma para atenderlo.

mod desk;
pub use desk::*;

pub use std::io::{BufRead, BufReader};
pub use std::net::TcpListener;
pub use std::path::{Path, PathBuf};
pub use std::process::{Child, Command, Stdio};
pub use std::sync::atomic::{AtomicUsize, Ordering};
pub use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
pub use std::sync::{Arc, Mutex};
pub use std::time::Duration;

pub use base64::engine::general_purpose::STANDARD;
pub use base64::Engine as _;

pub use rfirma_lib::desktop::adapters::paths::Paths;
pub use rfirma_lib::documents::ports::{DialogClues, PortalDialogs};
pub use rfirma_lib::identity::domain::store::Store;
pub use rfirma_lib::signing::adapters::isolate::Isolate;
pub use rfirma_lib::signing::adapters::tauri::signed_with_the_secret;
pub use rfirma_lib::signing::application::session::sign_on_token;
pub use rfirma_lib::signing::ports::{
    ProtectedSecret, SecretPromptError, SecretPromptRequest, SecretPrompter,
};
pub use rfirma_lib::site::adapters::channel::{bind_first_free, serve, SiteOperations};
pub use rfirma_lib::site::adapters::data_download::HttpDataSource;
pub use rfirma_lib::site::adapters::desk::Neighbours;
pub use rfirma_lib::site::adapters::relay::Relay;
pub use rfirma_lib::site::adapters::tls::LocalServerCertificate;
pub use rfirma_lib::site::application::errand::{
    self, Errand, ErrandDesk, ErrandStep, NegotiatedCodec, Transport,
};
pub use rfirma_lib::site::domain::channel::{ChannelDuty, ChannelLocation, OpenChannel};
pub use rfirma_lib::site::domain::local_ca::LocalCa;
pub use rfirma_lib::site::domain::protocol::{
    drawn_ports, read_operation, AfirmaUrl, LaunchRequest, NegotiatedCredential, SafCode,
    SiteOperation, WireAnswer, PROTOCOL_VERSION,
};
pub use rfirma_lib::site::domain::relay_error::{RelayError, Situation as RelaySituation};
pub use rfirma_lib::site::ports::{Inbox, ReplyHandle as ErrandReply, Servlets};
pub use rfirma_lib::Roots;

/// La versión de `service` que habla el cliente publicado cuando no hay WebSocket.
pub const THE_SERVICE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS: i64 = 1;

/// Tiempo máximo de espera para respuestas en pruebas.
pub const PATIENCE: Duration = Duration::from_secs(40);

/// La versión que el cliente publicado habla por defecto, y la que rfirma implementa.
pub const THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS: i64 = 4;

/// El guion de una sola selección, el de los casos que solo miran la invocación de arranque.
pub const THE_SINGLE_SELECTION: &str = "selectcert";

/// El guion de tres selecciones con el certificado fijado y soltado.
pub const THE_STICKY_SELECTIONS: &str = "sticky";

/// El guion de servidor intermedio que firma un documento demasiado largo para la URL.
pub const THE_RELAY_SIGNATURE: &str = "relay";

/// El guion de servidor intermedio que cofirma una factura: rFirma lo rechaza solo, sin
/// consentimiento, y el destino solo se conoce tras leer el XML de parámetros.
pub const THE_RELAY_REFUSED_OPERATION: &str = "relayrefused";

/// El guion del lote remoto: dos documentos firmados con `signBatchJSON`.
pub const THE_REMOTE_BATCH: &str = "batch";

/// El guion del lote remoto heredado: dos documentos firmados con `signBatch` en XML.
pub const THE_LEGACY_XML_BATCH: &str = "batchxml";

/// El guion del lote remoto sin presigner escuchando.
pub const THE_REMOTE_BATCH_WITH_THE_DOWN_PRESIGNER: &str = "batchdown";

/// El guion del lote local: `setLocalBatchProcess(true)` con un PDF, un binario y un XML.
pub const THE_LOCAL_BATCH: &str = "batchlocal";

/// El guion del lote local con el binario declarado `PAdES`, ilegible, y `stoponerror=true`.
pub const THE_LOCAL_BATCH_WITH_AN_ILLEGIBLE_ITEM: &str = "batchlocalillegible";

/// El guion de `sign` con `format=CAdES` y `mode=explicit` sobre el reto binario.
pub const THE_SIGN_CADES_EXPLICIT: &str = "signcades";

/// El guion de `sign` con `format=CAdES`, `mode=explicit` y `gzip=true` sobre el reto comprimido.
pub const THE_SIGN_GZIP: &str = "signgzip";

/// El guion de `sign` con `format=CAdES-ASiC-S` sobre el mismo reto binario.
pub const THE_SIGN_CADES_ASIC_S: &str = "signcadesasics";

/// El guion de `sign` con `format=auto` sobre el mismo reto binario.
pub const THE_SIGN_AUTO: &str = "signauto";

/// El guion de `sign` con `format=XAdES` sobre el XML de referencia.
pub const THE_SIGN_XADES: &str = "signxades";

/// El guion de `sign` con `format=auto` sobre el mismo XML de referencia.
pub const THE_SIGN_XADES_AUTO: &str = "signxadesauto";

/// El guion de `sign` con `format=PAdES` sobre el PDF que deja la prueba.
pub const THE_SIGN_PADES: &str = "signpades";

/// El mismo guion de `sign` con `format=PAdES`, con `checkSignatures=true` en las `properties`.
pub const THE_SIGN_PADES_CHECKING_SIGNATURES: &str = "signpadeschecking";

/// El guion de `sign` con `format=FacturaE` sobre la factura de referencia.
pub const THE_SIGN_FACTURAE: &str = "signfacturae";

/// El guion de `cosign` con `format=FacturaE` sobre la misma factura.
pub const THE_COSIGN_FACTURAE: &str = "cosignfacturae";

/// El guion de `saveDataToFile` sobre el reto de referencia.
pub const THE_SAVE: &str = "save";

/// El guion de `getFileNameContentBase64` para cargar un único fichero.
pub const THE_LOAD: &str = "load";

/// El guion de `getMultiFileNameContentBase64` para cargar varios ficheros.
pub const THE_MULTI_LOAD: &str = "multiload";

/// El guion de `signAndSaveToFile` sobre el reto binario con formato CAdES.
pub const THE_SIGN_AND_SAVE: &str = "signandsave";

/// El certificado de pruebas de la FNMT vigente del token `rfirma-test`.
pub const THE_TEST_CERTIFICATE: &str = "FNMT-ACTIVO-99999999R";

/// El secreto del token de pruebas `rfirma-test`.
pub const THE_TOKEN_SECRET: &str = "1234";

/// Intentos de atar la ubicación del canal antes de darla por ocupada.
pub const PORT_ATTEMPTS: usize = 60;

/// Los casos que se turnan: la confianza de la CA local con la que sirven los servlets del lote es
/// del proceso entero.
pub static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Modo en el que se fuerza al `autoscript.js` publicado a hablar, porque nunca manda `v=3` por
/// websocket por su cuenta.
#[derive(Clone, Copy)]
pub enum BenchMode {
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
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Fourth => "v4",
            Self::Third => "v3",
            Self::Service => "service",
            Self::Relay => "relay",
        }
    }
}

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
pub fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
pub fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/site-driver/driver.mjs")
}

/// Comprueba disponibilidad de Node y del script de autoscript.js.
pub fn the_bench_can_be_mounted() -> bool {
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
pub struct Event(pub String);

impl Event {
    pub fn name(&self) -> &str {
        self.field("event")
    }

    /// Extrae el valor de un campo del objeto JSON del evento.
    pub fn field(&self, name: &str) -> &str {
        let needle = format!("\"{name}\":\"");
        let Some(from) = self.0.find(&needle) else {
            return "";
        };
        let rest = &self.0[from + needle.len()..];
        rest.split('"').next().unwrap_or("")
    }
}

/// El cliente publicado corriendo bajo Node, con su salida ya en cola.
pub struct PublishedClient {
    child: Child,
    events: Receiver<Event>,
    pub third_port: Option<u16>,
}

impl PublishedClient {
    /// Arranca el conductor con la CA local en NODE_EXTRA_CA_CERTS, en el modo por defecto (v4).
    pub fn running_against(material: &ChannelMaterial) -> Self {
        Self::running_as(material, BenchMode::Fourth)
    }

    /// Arranca el conductor con la CA local en NODE_EXTRA_CA_CERTS, en el modo indicado.
    pub fn running_as(material: &ChannelMaterial, mode: BenchMode) -> Self {
        Self::running_the_script(material, mode, THE_SINGLE_SELECTION)
    }

    /// Arranca el conductor con uno de los guiones del banco, y con el material con el que sus
    /// servlets sirven TLS.
    pub fn running_the_script(material: &ChannelMaterial, mode: BenchMode, script: &str) -> Self {
        Self::spawn(material, mode, script, &[], a_free_port())
    }

    /// Arranca el conductor con uno de los guiones que necesitan el PDF que la prueba deja en
    /// disco: el del lote local y los de `sign` con `format=PAdES`.
    pub fn running_the_script_over_the_pdf(
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

    pub fn spawn(
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
    pub fn the_channel_location(&self, launch: &LaunchRequest) -> ChannelLocation {
        match self.third_port {
            Some(port) => ChannelLocation::Fixed(port),
            None => launch.location().clone(),
        }
    }

    /// Siguiente evento emitido por el cliente publicado, saltandose las observaciones de la suite.
    pub fn next_event(&self) -> Event {
        loop {
            let event = match self.events.recv_timeout(PATIENCE) {
                Ok(event) => event,
                Err(RecvTimeoutError::Timeout) => {
                    panic!("el cliente publicado no dijo nada en {PATIENCE:?}")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("el conductor murio sin dar un veredicto")
                }
            };
            if event.name() != "condition" {
                return event;
            }
        }
    }

    /// URL afirma:// construida por el cliente publicado.
    pub fn the_launch_url(&self) -> String {
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
pub struct ChannelMaterial {
    certificate: LocalServerCertificate,
    pub ca_pem_file: tempfile::NamedTempFile,
    certificate_pem_file: tempfile::NamedTempFile,
    key_pem_file: tempfile::NamedTempFile,
}

impl ChannelMaterial {
    pub fn fresh() -> Self {
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
pub fn a_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("deberia haber un puerto de loopback libre")
        .port()
}

/// Un fichero temporal con `bytes` ya en disco, con el `suffix` indicado.
pub fn a_temp_file(suffix: &str, bytes: &[u8]) -> tempfile::NamedTempFile {
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
pub fn a_pem_file(pem: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".pem", pem)
}

/// Un fichero temporal con el DER ya en disco, que es como lo lee OpenSSL.
pub fn a_der_file(der: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".der", der)
}

/// Un fichero temporal con el XML ya en disco, que es como lo leen `xmllint` y el oráculo.
pub fn an_xml_file(xml: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".xml", xml)
}

/// Un fichero temporal con el contenedor ASiC-S ya en disco, que es como lo lee el oráculo.
pub fn an_asic_s_file(container: &[u8]) -> tempfile::NamedTempFile {
    a_temp_file(".asics", container)
}

/// Ruta del reto de 64 bytes del banco de referencia, el que firma el guion `sign`.
pub fn the_challenge_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/reference/challenge.bin")
}

/// El binario del lote local: nunca empieza por `%PDF-`, así que declararlo `format=PAdES` es lo
/// que lo vuelve ilegible en el guion de `stoponerror`.
pub const THE_LOCAL_BATCH_BINARY: &[u8] =
    b"contenido binario del lote local, sin PDF ni XML dentro";

/// Genera un PDF sintético de una página, admisible para `format=PAdES` (ADR-0014).
pub fn a_one_page_pdf() -> Vec<u8> {
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
pub const PDFSIG_VALID: &str = "Signature Validation: Signature is Valid.";

/// Valida la firma PAdES con `pdfsig` (ADR-0014).
pub fn validated_by_pdfsig(pdf: &Path) {
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
pub async fn the_channel_on_one_of(
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
pub fn no_operations() -> SiteOperations {
    SiteOperations::for_operations(|_, _| {})
}

/// Abre el canal en la ubicación indicada: uno de los puertos sorteados, o el puerto fijo del
/// protocolo 3.
pub async fn the_channel_at(
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
pub async fn bound_once_free(location: &ChannelLocation) -> TcpListener {
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
