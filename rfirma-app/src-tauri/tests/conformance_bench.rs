//! **El banco de conformidad**: el `autoscript.js` publicado —el que sirve una
//! sede de verdad— hablando con el canal de rfirma bajo Node (TD-55, TD-59).
//!
//! No se confunde con el **cliente de canal** (`tests/channel_client.rs`), que
//! es el propio, en Rust: aquel provoca los rechazos que un cliente conforme no
//! puede provocar; este comprueba que el cliente **real** entiende lo que
//! contestamos. Son dos cosas con dos trabajos distintos (ID-311).
//!
//! **Es grada B, no D** (ID-312, ADR-0014). El sujeto de la prueba es el canal
//! de rfirma, y el `autoscript.js` es un accesorio fijo: se descarga una vez a
//! etiqueta fijada y con `sha256` comprobado —`just autoscript`— y a partir de
//! ahí es un fichero en disco, como el kit de la FNMT. Nada de esto depende de
//! que un tercero esté vivo cuando la prueba corre.
//!
//! **Si falta, falla en el CI y sólo se salta en local** (TD-55): con `CI`
//! puesta, la ausencia del accesorio o de Node es un fallo; sin ella, la prueba
//! se salta diciendo qué ejecutar.
//!
//! El accesorio **no se copia al repositorio**: es EUPL-1.1/GPL-2.0 ajeno y
//! `testdata/conformance/` está fuera del control de versiones.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::Duration;

use rfirma_lib::channel::{bind_first_free, serve, ChannelDuty, OpenChannel};
use rfirma_lib::protocol::{drawn_ports, AfirmaUrl, LaunchRequest, SafCode, PROTOCOL_VERSION};
use rfirma_lib::tls::{LocalCa, LocalServerCertificate};

/// Lo que tarda de más un veredicto que no va a llegar. El propio
/// `autoscript.js` espera 3 s antes del primer intento y reintenta cada 2 s.
const PATIENCE: Duration = Duration::from_secs(40);

/// La versión que el cliente publicado habla, y la que rfirma implementa.
const THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS: i64 = 4;

// ---------------------------------------------------------------------------
// El accesorio
// ---------------------------------------------------------------------------

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/driver.mjs")
}

/// Si el banco no se puede montar: en el CI es un fallo, en local un aviso.
///
/// Ésta es la mitad del TD-55 que impide que el banco se apague solo. Un
/// `skip` silencioso en el CI convertiría la prueba en decoración el día que
/// la descarga fallase.
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

// ---------------------------------------------------------------------------
// El banco
// ---------------------------------------------------------------------------

/// Un evento del conductor: una línea de JSON de su salida estándar.
///
/// No se deserializa a un tipo: lo que la prueba necesita son tres campos de
/// texto, y traer `serde_json` al banco por eso sería más andamio que prueba.
struct Event(String);

impl Event {
    fn name(&self) -> &str {
        self.field("event")
    }

    /// El valor de un campo del objeto JSON, tal cual, sin escapes que
    /// deshacer: ni la URL de invocación ni un `SAF_NN` los llevan.
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
    /// Arranca el conductor con la CA local en `NODE_EXTRA_CA_CERTS`, que es
    /// como una sede de verdad confía en el certificado del servidor local.
    fn running_against(ca_pem_path: &std::path::Path) -> Self {
        let mut child = Command::new("node")
            .arg(the_driver())
            .env("RFIRMA_AUTOSCRIPT", the_published_client())
            .env("NODE_EXTRA_CA_CERTS", ca_pem_path)
            .env("RFIRMA_BENCH_TIMEOUT_MS", PATIENCE.as_millis().to_string())
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

    /// El siguiente evento, o el porqué de no haberlo.
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

    /// La URL `afirma://` que el cliente publicado construye para arrancar
    /// rfirma. Es el primer evento, siempre.
    fn the_launch_url(&self) -> String {
        let event = self.next_event();
        assert_eq!(
            event.name(),
            "launch",
            "el primer evento del banco es la invocacion"
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

/// El material del canal: la CA local, el certificado del servidor local y el
/// PEM de la CA en un fichero que Node pueda leer.
struct ChannelMaterial {
    certificate: LocalServerCertificate,
    ca_pem_file: tempfile::NamedTempFile,
}

impl ChannelMaterial {
    fn fresh() -> Self {
        use std::io::Write;

        let ca = LocalCa::generate().expect("la CA local deberia generarse");
        let certificate =
            LocalServerCertificate::issued_by(&ca).expect("el certificado deberia emitirse");
        let mut ca_pem_file = tempfile::Builder::new()
            .suffix(".pem")
            .tempfile()
            .expect("un fichero temporal para la CA");
        ca_pem_file
            .write_all(&ca.certificate_pem().expect("la CA local en PEM"))
            .expect("la CA deberia escribirse");
        ca_pem_file.flush().expect("la CA deberia quedar en disco");

        Self {
            certificate,
            ca_pem_file,
        }
    }
}

/// Ata el canal en uno de los puertos que el cliente publicado sorteó de
/// verdad, con el cometido que se le dé (ID-215).
async fn the_channel_on_one_of(
    url: &AfirmaUrl,
    material: &ChannelMaterial,
    duty: ChannelDuty,
) -> OpenChannel {
    let listener = bind_first_free(&drawn_ports(url))
        .expect("alguno de los tres sorteados deberia estar libre");
    serve(
        listener,
        &material.certificate,
        duty,
        std::sync::Arc::new(|_, _| {}),
    )
    .await
    .expect("el canal deberia levantarse")
}

// ---------------------------------------------------------------------------
// Las pruebas
// ---------------------------------------------------------------------------

/// **La invocación real cabe en nuestro lector** (ID-245…ID-249): la URL que
/// construye el cliente publicado —no una escrita a mano en una prueba— la
/// entiende `LaunchRequest`, con sus tres puertos y su credencial de canal.
///
/// Sin esta, todo lo que sabemos del formato viene de leer el fichero.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_url_the_published_client_builds_is_the_one_rfirma_reads() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(material.ca_pem_file.path());
    let url = client.the_launch_url();

    let parsed =
        AfirmaUrl::parse(&url).expect("la invocacion del cliente publicado deberia leerse");
    let launch = LaunchRequest::from_url(&parsed).expect("la version 4 se habla aqui");

    assert_eq!(
        PROTOCOL_VERSION, THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS,
        "rfirma implementa la version que el cliente publicado envia"
    );
    assert_eq!(
        launch.ports().len(),
        3,
        "el cliente publicado sortea tres puertos, y llegaron {:?}",
        launch.ports()
    );
    assert_eq!(
        launch.credential().as_str().len(),
        20,
        "la credencial de canal son veinte alfanumericos"
    );
}

/// **El `SAF_21` de versión no soportada llega al `errorCallback`, y el cliente
/// publicado no se cuelga** (TD-59).
///
/// Es la única forma de saberlo: el rechazo por versión lo decide rfirma y sale
/// al primer mensaje del canal (ID-248), que en el cliente real es la petición
/// de eco. Lo que hace con él no se puede deducir leyendo el fichero.
///
/// **Lo medido**: el cliente entrega un error y termina. No entrega el `SAF_21`
/// literal, porque su `onMessageEchoFunction` no mira la respuesta del eco:
/// la toma por buena, manda la operación al canal ya cerrado y es el cierre lo
/// que dispara el `errorCallback`. La sede se entera de que el trámite no sigue
/// —que es lo que el repliegue escrito perseguía—, así que **el repliegue no se
/// aplica**: no hace falta cerrar el socket sin contestar, y contestando queda
/// además el código en el registro del navegador.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unsupported_version_reaches_the_error_callback_of_the_published_client() {
    if !the_bench_can_be_mounted() {
        return;
    }

    let material = ChannelMaterial::fresh();
    let client = PublishedClient::running_against(material.ca_pem_file.path());
    let url = client.the_launch_url();

    // El mismo trámite, pero declarando una versión que rfirma no habla. Se
    // construye sobre la URL real para que lo único distinto sea la versión.
    let unsupported = url.replace(
        &format!("&v={THE_VERSION_THE_PUBLISHED_CLIENT_SPEAKS}"),
        "&v=3",
    );
    let refusal = LaunchRequest::parse(&unsupported).expect_err("la version 3 no se habla aqui");
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
