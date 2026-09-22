//! El trámite y el conductor de la suite de conformidad: arrancar el cliente publicado, leer
//! sus eventos, invocar al cliente y extraer lo que cada evento trae.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::catalogue::Check;
use crate::livelog::{LiveLogSink, Provenance};
use crate::outcome::Outcome;
use crate::transcript::{legible, Transcript};
use crate::Probe;

/// El `type` con el que el conductor avisa de que reventó él, no el cliente.
pub(crate) const THE_DRIVER_CRASH: &str = "uncaught";

/// Lo que se anota cuando al conductor se le acaba la paciencia sin que nadie responda.
pub(crate) const THE_EXHAUSTED_PATIENCE: &str = "timeout";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProtocolConditionResult {
    pub(crate) name: String,
    pub(crate) outcome: Outcome,
    pub(crate) observation: Option<String>,
}

/// Lo que se pudo medir de un trámite: si el cliente llegó a arrancar, el código SAF que emitió,
/// la clase con la que el cliente publicado lo envolvió, y la firma o datos que devolvió si hubo éxito.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct ErrandOutcome {
    pub(crate) launched: bool,
    pub(crate) error_type: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) data: Option<String>,
    pub(crate) protocol_conditions: Vec<ProtocolConditionResult>,
}

impl ErrandOutcome {
    /// Si se le acabó la paciencia al conductor: eso no es una observación que valga guardar.
    pub(crate) fn exhausted_its_patience(&self) -> bool {
        self.error_type.as_deref() == Some(THE_EXHAUSTED_PATIENCE)
    }
}

/// Lo que determina un trámite sin persona delante: modo, guion, almacén y el arnés si lo hay.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct ErrandKey(String);

impl ErrandKey {
    /// `None` si no se conduce o si lo que viaja depende de lo que haga la persona en el diálogo.
    pub(crate) fn of(check: &Check) -> Option<Self> {
        let drive = check.drive.as_ref()?;
        if check.needs_a_person() {
            return None;
        }
        let mut key = format!("{}/{}/{}", drive.mode, drive.script, check.store.name());
        if let Some(harness) = check.harness {
            key = format!("{key}/{}", harness.name);
        }
        Some(Self(key))
    }
}

/// Un trámite observado tal y como se guarda en el informe: lo medido, la comprobación cuyas
/// tramas lo transcriben y lo que tardó.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ObservedErrand {
    pub(crate) outcome: ErrandOutcome,
    pub(crate) transcribed_in: String,
    pub(crate) duration_ms: u64,
}

impl ObservedErrand {
    pub(crate) fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }
}

/// Quien corre los trámites: Node con el cliente publicado, o transcripciones grabadas en las
/// pruebas.
pub(crate) trait ErrandRunner: Send + Sync {
    fn run(
        &self,
        probe: &Probe,
        transcript_name: &str,
        script: &str,
        mode: &str,
        patience: Duration,
    ) -> ErrandOutcome;
}

pub(crate) struct NodeRunner;

impl ErrandRunner for NodeRunner {
    /// Corre el trámite bajo Node, transcribiendo cada evento del cliente publicado a medida que
    /// llega; al terminar mata al cliente, respondiera o no.
    fn run(
        &self,
        probe: &Probe,
        transcript_name: &str,
        script: &str,
        mode: &str,
        patience: Duration,
    ) -> ErrandOutcome {
        let witness = &probe.witness;
        let start = witness.started_at();
        let log_sink = witness.log_sink();
        let mut transcript = match Transcript::open(&probe.report, transcript_name) {
            Ok(transcript) => Some(transcript),
            Err(complaint) => {
                witness.harness(&complaint);
                None
            }
        };
        let trust_root = the_trust_root_as_pem(&probe.trust_root);
        let mut driver = the_published_client_running(trust_root.path(), patience, script, mode);
        witness.driver_spawned(driver.id());
        let events = driver.stdout.take().expect("el conductor escribe eventos");
        let mut client: Option<ClientProcess> = None;
        let outcome = observe(
            BufReader::new(events).lines().map_while(Result::ok),
            |event| {
                if let Some(Err(complaint)) = transcript.as_mut().map(|t| t.record(event)) {
                    witness.harness(&complaint);
                    transcript = None;
                }
                log_sink.push(Provenance::Site, start.elapsed(), &legible(event));
                if let Some(url) = the_launch_url_in(event) {
                    log_sink.push(
                        Provenance::Suite,
                        start.elapsed(),
                        &format!("invoco {} con {url}", probe.client.display()),
                    );
                    client = Some(ClientProcess::spawn(
                        &probe.client,
                        &url,
                        log_sink.clone(),
                        start,
                    ));
                }
            },
        );
        let _ = driver.wait();
        witness.driver_finished();
        if let Some(mut client) = client {
            client.terminate();
        }
        outcome
    }
}

impl Probe {
    /// Corre `script` en `mode` contra el cliente declarado y devuelve lo que se pudo medir.
    pub(crate) fn run_errand(
        &self,
        transcript_name: &str,
        script: &str,
        mode: &str,
        patience: Duration,
    ) -> ErrandOutcome {
        self.runner
            .run(self, transcript_name, script, mode, patience)
    }
}

/// Lo que dicen los eventos de un trámite, pasándole cada uno a `each` según llega.
pub(crate) fn observe(
    events: impl IntoIterator<Item = String>,
    mut each: impl FnMut(&str),
) -> ErrandOutcome {
    let mut outcome = ErrandOutcome::default();
    for event in events {
        each(&event);
        outcome.launched |= the_launch_url_in(&event).is_some();
        if let Some(kind) = the_error_type_in(&event) {
            outcome.error_type = Some(kind);
        }
        if let Some(code) = the_saf_code_in(&event) {
            outcome.error_code = Some(code);
        }
        if let Some(result) = the_signature_in(&event) {
            outcome.signature = Some(result);
        }
        if let Some(result) = the_data_in(&event) {
            outcome.data = Some(result);
        }
        if let Some(condition) = the_protocol_condition_in(&event) {
            outcome.protocol_conditions.push(condition);
        }
        if event.contains("\"event\":\"timeout\"") {
            outcome.error_type = Some(THE_EXHAUSTED_PATIENCE.to_owned());
        }
    }
    outcome
}

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
pub(crate) fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
pub(crate) fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../testdata/site-driver/driver.mjs")
}

fn the_published_client_running(
    trust_root: &Path,
    patience: Duration,
    script: &str,
    mode: &str,
) -> Child {
    let published_client = the_published_client();
    assert!(
        published_client.exists(),
        "falta {}: ejecuta `just autoscript`",
        published_client.display()
    );
    Command::new("node")
        .arg(the_driver())
        .env("RFIRMA_AUTOSCRIPT", published_client)
        .env("NODE_EXTRA_CA_CERTS", trust_root)
        .env("RFIRMA_BENCH_TIMEOUT_MS", patience.as_millis().to_string())
        .env("RFIRMA_BENCH_MODE", mode)
        .env("RFIRMA_BENCH_SCRIPT", script)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Node debería arrancar el conductor")
}

pub(crate) struct ClientProcess {
    child: Child,
    drain_handles: Vec<JoinHandle<()>>,
}

impl ClientProcess {
    pub(crate) fn spawn(client: &Path, url: &str, log_sink: LiveLogSink, start: Instant) -> Self {
        let mut child = Command::new(client)
            .arg(url)
            .process_group(0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{} no arrancó: {error}", client.display()));

        let mut drain_handles = Vec::new();
        if let Some(stdout) = child.stdout.take() {
            let sink = log_sink.clone();
            drain_handles.push(spawn(move || drain_stream(stdout, sink, start)));
        }
        if let Some(stderr) = child.stderr.take() {
            let sink = log_sink.clone();
            drain_handles.push(spawn(move || drain_stream(stderr, sink, start)));
        }
        Self {
            child,
            drain_handles,
        }
    }

    /// Mata al cliente con todo su grupo: el lanzador deja vivo el `java` que hereda las tuberías.
    pub(crate) fn terminate(&mut self) {
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", self.child.id())])
            .status();
        let _ = self.child.kill();
        let _ = self.child.wait();
        for handle in self.drain_handles.drain(..) {
            let _ = handle.join();
        }
    }
}

fn drain_stream(stream: impl std::io::Read, log_sink: LiveLogSink, start: Instant) {
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        log_sink.push(Provenance::Client, start.elapsed(), &line);
    }
}

/// `NODE_EXTRA_CA_CERTS` solo lee PEM, y la raíz que instala AutoFirma en el escritorio es DER.
fn the_trust_root_as_pem(certificate: &Path) -> tempfile::NamedTempFile {
    let bytes = std::fs::read(certificate)
        .unwrap_or_else(|error| panic!("{} no se pudo leer: {error}", certificate.display()));
    let parsed = openssl::x509::X509::from_pem(&bytes)
        .or_else(|_| openssl::x509::X509::from_der(&bytes))
        .unwrap_or_else(|_| panic!("{} no es un certificado PEM ni DER", certificate.display()));
    let mut file = tempfile::NamedTempFile::new().expect("el PEM temporal debería crearse");
    file.write_all(
        &parsed
            .to_pem()
            .expect("el certificado debería volver a PEM"),
    )
    .expect("el PEM temporal debería escribirse");
    file
}

fn the_launch_url_in(event: &str) -> Option<String> {
    if !event.contains("\"event\":\"launch\"") {
        return None;
    }
    let needle = "\"url\":\"";
    let from = event.find(needle)? + needle.len();
    Some(event[from..].split('"').next()?.to_owned())
}

fn the_error_type_in(event: &str) -> Option<String> {
    if !event.contains("\"event\":\"error\"") {
        return None;
    }
    let needle = "\"type\":\"";
    let from = event.find(needle)? + needle.len();
    Some(event[from..].split('"').next()?.to_owned())
}

fn the_error_message_in(event: &str) -> Option<String> {
    if !event.contains("\"event\":\"error\"") {
        return None;
    }
    let needle = "\"message\":\"";
    let from = event.find(needle)? + needle.len();
    Some(event[from..].split('"').next()?.to_owned())
}

/// La firma en Base64 de un evento de éxito, que viaja en el campo `result`.
fn the_signature_in(event: &str) -> Option<String> {
    if !event.contains("\"event\":\"success\"") {
        return None;
    }
    let needle = "\"result\":\"";
    let from = event.find(needle)? + needle.len();
    Some(event[from..].split('"').next()?.to_owned())
}

/// Los datos en Base64 o certificado de un evento de éxito, que viajan en el campo `data`.
fn the_data_in(event: &str) -> Option<String> {
    if !event.contains("\"event\":\"success\"") {
        return None;
    }
    let needle = "\"data\":\"";
    let from = event.find(needle)? + needle.len();
    Some(event[from..].split('"').next()?.to_owned())
}

/// El código SAF del error, que viaja en el `message`: el `type` solo trae la clase que lo
/// envolvió.
fn the_saf_code_in(event: &str) -> Option<String> {
    let message = the_error_message_in(event)?;
    let code: String = message
        .chars()
        .take_while(|letter| {
            letter.is_ascii_uppercase() || letter.is_ascii_digit() || *letter == '_'
        })
        .collect();
    code.starts_with("SAF_").then_some(code)
}

fn the_protocol_condition_in(event: &str) -> Option<ProtocolConditionResult> {
    if !event.contains("\"event\":\"condition\"") {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(event).ok()?;
    let name = value.get("name")?.as_str()?.to_owned();
    let outcome_str = value.get("verdict")?.as_str()?;
    let outcome = match outcome_str {
        "compliant" => Outcome::Compliant,
        "discrepant" => Outcome::Noncompliant,
        _ => Outcome::NotObservable,
    };
    let observation = value
        .get("observation")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    Some(ProtocolConditionResult {
        name,
        outcome,
        observation,
    })
}

/// Las tramas grabadas de un trámite de verdad, en `tests/transcripts/`.
#[cfg(test)]
pub(crate) fn the_recorded(name: &str) -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/transcripts")
        .join(format!("{name}.jsonl"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} no se pudo leer: {error}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
pub(crate) mod fake {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{observe, ErrandOutcome, ErrandRunner};
    use crate::transcript::Transcript;
    use crate::Probe;

    /// Un ejecutor que no lanza nada: devuelve las tramas grabadas de cada guion y apunta cada
    /// trámite que se le pidió.
    #[derive(Default)]
    pub(crate) struct RecordedRunner {
        recorded: BTreeMap<String, Vec<String>>,
        runs: Mutex<Vec<String>>,
    }

    impl RecordedRunner {
        pub(crate) fn replaying(recorded: &[(&str, Vec<String>)]) -> Arc<Self> {
            Arc::new(Self {
                recorded: recorded
                    .iter()
                    .map(|(script, events)| ((*script).to_owned(), events.clone()))
                    .collect(),
                runs: Mutex::default(),
            })
        }

        /// Los trámites corridos, `modo/guion`, en el orden en que se pidieron.
        pub(crate) fn runs(&self) -> Vec<String> {
            self.runs.lock().unwrap().clone()
        }
    }

    impl ErrandRunner for RecordedRunner {
        fn run(
            &self,
            probe: &Probe,
            transcript_name: &str,
            script: &str,
            mode: &str,
            _: Duration,
        ) -> ErrandOutcome {
            self.runs.lock().unwrap().push(format!("{mode}/{script}"));
            let mut transcript = Transcript::open(&probe.report, transcript_name).ok();
            let events = self.recorded.get(script).cloned().unwrap_or_default();
            observe(events, |event| {
                if let Some(transcript) = transcript.as_mut() {
                    let _ = transcript.record(event);
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn reads_the_saf_code_from_the_message() {
        assert_eq!(
            the_saf_code_in(
                r#"{"event":"error","message":"SAF_47: Peticion al socket desde IP externa","type":"java.lang.Exception"}"#
            )
            .as_deref(),
            Some("SAF_47")
        );
    }

    #[test]
    fn ignores_a_message_without_a_saf_code() {
        assert_eq!(
            the_saf_code_in(
                r#"{"event":"error","message":"Cannot read properties of null","type":"uncaught"}"#
            ),
            None
        );
    }

    #[test]
    fn ignores_an_event_that_is_not_an_error() {
        assert_eq!(
            the_saf_code_in(r#"{"event":"launch","url":"afirma://websocket?v=4"}"#),
            None
        );
    }

    #[test]
    fn reads_the_signature_from_a_success_event() {
        assert_eq!(
            the_signature_in(r#"{"event":"success","result":"TUlJQg==","certificate":"x"}"#)
                .as_deref(),
            Some("TUlJQg==")
        );
    }

    #[test]
    fn reads_the_data_from_a_success_event() {
        assert_eq!(
            the_data_in(r#"{"event":"success","data":"TUlJQg=="}"#).as_deref(),
            Some("TUlJQg==")
        );
    }

    #[test]
    fn ignores_an_event_that_is_not_a_success() {
        assert_eq!(
            the_signature_in(
                r#"{"event":"error","message":"SAF_03: parametro invalido","type":"java.lang.Exception"}"#
            ),
            None
        );
    }

    #[test]
    fn reads_a_protocol_condition_event() {
        let event = r#"{"event":"condition","name":"the-echo-answers-ok","verdict":"compliant","observation":"OK"}"#;
        let condition = the_protocol_condition_in(event).expect("debería leer la condición");
        assert_eq!(condition.name, "the-echo-answers-ok");
        assert_eq!(condition.outcome, Outcome::Compliant);
        assert_eq!(condition.observation.as_deref(), Some("OK"));
    }

    #[test]
    fn ignores_an_event_without_protocol_condition() {
        let event = r#"{"event":"launch","url":"afirma://websocket"}"#;
        assert!(the_protocol_condition_in(event).is_none());
    }
    #[test]
    fn drain_stream_delivers_lines_as_the_client() {
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&delivered);
        let log_sink = LiveLogSink::new(move |line| into.lock().unwrap().push(line));
        let input = std::io::Cursor::new(b"linea 1\nlinea 2\nlinea 3\n");

        drain_stream(input, log_sink, Instant::now());

        let delivered = delivered.lock().unwrap();
        assert_eq!(delivered.len(), 3);
        assert!(delivered[0].ends_with("cliente linea 1"));
    }

    #[test]
    fn a_recorded_rejection_is_observed_as_launched_with_its_code() {
        let outcome = observe(the_recorded("a-rejection-with-saf-03"), |_| {});

        assert!(outcome.launched);
        assert_eq!(outcome.error_code.as_deref(), Some("SAF_03"));
        assert_eq!(outcome.error_type.as_deref(), Some("java.lang.Exception"));
    }

    #[test]
    fn a_timeout_event_is_observed_as_exhausted_patience() {
        let outcome = observe([r#"{"event":"timeout"}"#.to_owned()], |_| {});

        assert!(!outcome.launched);
        assert!(outcome.exhausted_its_patience());
    }

    #[test]
    fn each_event_is_handed_over_as_it_arrives() {
        let mut seen = Vec::new();

        observe(the_recorded("a-rejection-with-saf-03"), |event| {
            seen.push(event.to_owned());
        });

        assert_eq!(seen.len(), 2);
        assert!(seen[0].contains("\"launch\""));
    }

    fn a_check(extra: &str) -> Check {
        crate::catalogue::the_catalogue_in(&format!(
            "[[check]]\nid = \"a\"\nset = \"errores\"\nchapter = \"15\"\ncitation = \"A.java:1\"\n\
             statement = \"Algo.\"\ndrive = {{ mode = \"v4\", script = \"save\" }}\n{extra}"
        ))
        .unwrap()
        .remove(0)
    }

    #[test]
    fn the_key_of_an_errand_is_its_mode_script_store_and_harness() {
        assert_eq!(
            ErrandKey::of(&a_check("assistance = \"click\"\nstore = \"ec\"")),
            Some(ErrandKey("v4/save/ec".to_owned()))
        );
        assert_eq!(
            ErrandKey::of(&a_check(
                "assistance = \"none\"\nharness = \"occupied_service_ports\""
            )),
            Some(ErrandKey("v4/save/rsa/occupied_service_ports".to_owned()))
        );
    }

    #[test]
    fn an_errand_that_depends_on_what_the_person_does_has_no_key() {
        assert_eq!(ErrandKey::of(&a_check("assistance = \"person\"")), None);
    }
}
