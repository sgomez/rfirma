//! El trámite y el conductor de la suite de conformidad: arrancar el cliente publicado, leer
//! sus eventos, invocar al sujeto y extraer lo que cada evento trae.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

use crate::dossier::Verdict;
use crate::livelog::{LiveLogSink, Provenance};
use crate::transcript::{legible, Transcript};
use crate::Probe;

/// El `type` con el que el conductor avisa de que reventó él, no el sujeto.
pub(crate) const THE_DRIVER_CRASH: &str = "uncaught";

/// Lo que se anota cuando al conductor se le acaba la paciencia sin que nadie responda.
pub(crate) const THE_EXHAUSTED_PATIENCE: &str = "timeout";

#[derive(Debug, Clone)]
pub(crate) struct ProtocolConditionResult {
    pub(crate) id: String,
    pub(crate) verdict: Verdict,
    pub(crate) observation: Option<String>,
}

/// Lo que se pudo medir de un trámite: si el sujeto llegó a arrancar, el código SAF que emitió,
/// la clase con la que el cliente publicado lo envolvió, y la firma o datos que devolvió si hubo éxito.
pub(crate) struct ErrandOutcome {
    pub(crate) launched: bool,
    pub(crate) error_type: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) signature: Option<String>,
    pub(crate) data: Option<String>,
    pub(crate) protocol_conditions: Vec<ProtocolConditionResult>,
    #[allow(dead_code)]
    pub(crate) recent_subject_lines: Vec<String>,
}

impl Probe {
    /// Corre `script` en `mode` contra el sujeto declarado, transcribiendo cada evento del
    /// cliente publicado a medida que llega, y devuelve lo que se pudo medir del trámite.
    pub(crate) fn run_errand(
        &self,
        transcript_name: &str,
        script: &str,
        mode: &str,
        patience: Duration,
    ) -> ErrandOutcome {
        let start = self.witness.started_at();
        let log_sink = self.witness.log_sink();
        let mut transcript = match Transcript::open(&self.report, transcript_name) {
            Ok(transcript) => Some(transcript),
            Err(complaint) => {
                self.witness.harness(&complaint);
                None
            }
        };
        let trust_root = the_trust_root_as_pem(&self.trust_root);
        let mut driver = the_published_client_running(trust_root.path(), patience, script, mode);
        self.witness.driver_spawned(driver.id());
        let events = driver.stdout.take().expect("el conductor escribe eventos");
        let mut subject: Option<SubjectProcess> = None;
        let mut error_type = None;
        let mut error_code = None;
        let mut signature = None;
        let mut data = None;
        let mut protocol_conditions = Vec::new();
        for event in BufReader::new(events).lines().map_while(Result::ok) {
            if let Some(Err(complaint)) = transcript.as_mut().map(|t| t.record(&event)) {
                self.witness.harness(&complaint);
                transcript = None;
            }
            log_sink.push(Provenance::Driver, start.elapsed(), &legible(&event));
            if let Some(url) = the_launch_url_in(&event) {
                log_sink.push(
                    Provenance::Harness,
                    start.elapsed(),
                    &format!("invoco {} con {url}", self.subject.display()),
                );
                subject = Some(SubjectProcess::spawn(
                    &self.subject,
                    &url,
                    log_sink.clone(),
                    start,
                ));
            }
            if let Some(kind) = the_error_type_in(&event) {
                error_type = Some(kind);
            }
            if let Some(code) = the_saf_code_in(&event) {
                error_code = Some(code);
            }
            if let Some(result) = the_signature_in(&event) {
                signature = Some(result);
            }
            if let Some(result) = the_data_in(&event) {
                data = Some(result);
            }
            if let Some(condition) = the_protocol_condition_in(&event) {
                protocol_conditions.push(condition);
            }
            if event.contains("\"event\":\"timeout\"") {
                error_type = Some(THE_EXHAUSTED_PATIENCE.to_owned());
            }
        }
        let launched = subject.is_some();
        let _ = driver.wait();
        self.witness.driver_finished();
        let recent_subject_lines = if let Some(mut subject_proc) = subject {
            let recents = subject_proc.recent_lines();
            subject_proc.terminate();
            recents
        } else {
            Vec::new()
        };
        ErrandOutcome {
            launched,
            error_type,
            error_code,
            signature,
            data,
            protocol_conditions,
            recent_subject_lines,
        }
    }
}

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
pub(crate) fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/driver.mjs")
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

pub(crate) struct SubjectProcess {
    child: Child,
    recent_lines: Arc<Mutex<VecDeque<String>>>,
    drain_handles: Vec<JoinHandle<()>>,
}

impl SubjectProcess {
    pub(crate) fn spawn(subject: &Path, url: &str, log_sink: LiveLogSink, start: Instant) -> Self {
        let mut child = Command::new(subject)
            .arg(url)
            .process_group(0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("{} no arrancó: {error}", subject.display()));

        let recent_lines = Arc::new(Mutex::new(VecDeque::with_capacity(64)));
        let mut drain_handles = Vec::new();

        if let Some(stdout) = child.stdout.take() {
            let sink = log_sink.clone();
            let recents = Arc::clone(&recent_lines);
            drain_handles.push(spawn(move || drain_stream(stdout, sink, start, recents)));
        }

        if let Some(stderr) = child.stderr.take() {
            let sink = log_sink.clone();
            let recents = Arc::clone(&recent_lines);
            drain_handles.push(spawn(move || drain_stream(stderr, sink, start, recents)));
        }

        Self {
            child,
            recent_lines,
            drain_handles,
        }
    }

    pub(crate) fn recent_lines(&self) -> Vec<String> {
        self.recent_lines.lock().unwrap().iter().cloned().collect()
    }

    /// Mata al sujeto con todo su grupo: el lanzador deja vivo el `java` que hereda las tuberías.
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

fn drain_stream(
    stream: impl std::io::Read,
    log_sink: LiveLogSink,
    start: Instant,
    recents: Arc<Mutex<VecDeque<String>>>,
) {
    let reader = BufReader::new(stream);
    for line in reader.lines().map_while(Result::ok) {
        log_sink.push(Provenance::Subject, start.elapsed(), &line);
        if let Ok(mut recents_lock) = recents.lock() {
            if recents_lock.len() >= 50 {
                recents_lock.pop_front();
            }
            recents_lock.push_back(line);
        }
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
    let id = value.get("id")?.as_str()?.to_owned();
    let verdict_str = value.get("verdict")?.as_str()?;
    let verdict = match verdict_str {
        "compliant" => Verdict::Compliant,
        "discrepant" => Verdict::Noncompliant,
        _ => Verdict::NotObservable,
    };
    let observation = value
        .get("observation")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    Some(ProtocolConditionResult {
        id,
        verdict,
        observation,
    })
}

#[cfg(test)]
mod tests {
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
        let event = r#"{"event":"condition","id":"v4_echo_greeting","verdict":"compliant","observation":"OK"}"#;
        let condition = the_protocol_condition_in(event).expect("debería leer la condición");
        assert_eq!(condition.id, "v4_echo_greeting");
        assert_eq!(condition.verdict, Verdict::Compliant);
        assert_eq!(condition.observation.as_deref(), Some("OK"));
    }

    #[test]
    fn ignores_an_event_without_protocol_condition() {
        let event = r#"{"event":"launch","url":"afirma://websocket"}"#;
        assert!(the_protocol_condition_in(event).is_none());
    }
    #[test]
    fn drain_stream_delivers_lines_as_the_subject_and_preserves_recents() {
        let delivered = Arc::new(Mutex::new(Vec::new()));
        let into = Arc::clone(&delivered);
        let log_sink = LiveLogSink::new(move |line| into.lock().unwrap().push(line));
        let recents = Arc::new(Mutex::new(VecDeque::new()));
        let input = std::io::Cursor::new(b"linea 1\nlinea 2\nlinea 3\n");

        drain_stream(input, log_sink, Instant::now(), Arc::clone(&recents));

        let stored: Vec<String> = recents.lock().unwrap().iter().cloned().collect();
        assert_eq!(stored, vec!["linea 1", "linea 2", "linea 3"]);

        let delivered = delivered.lock().unwrap();
        assert_eq!(delivered.len(), 3);
        assert!(delivered[0].ends_with("sujeto    linea 1"));
    }
    #[test]
    fn outcome_carries_recent_subject_lines() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: None,
            error_code: None,
            signature: None,
            data: None,
            protocol_conditions: Vec::new(),
            recent_subject_lines: vec!["line 1".to_string()],
        };
        assert_eq!(outcome.recent_subject_lines.len(), 1);
    }
}
