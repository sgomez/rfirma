//! El trámite y el conductor del sondeo: arrancar el cliente publicado, leer sus eventos,
//! invocar al sujeto y extraer lo que cada evento trae.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::transcript::Transcript;
use crate::Probe;

/// El `type` con el que el conductor avisa de que reventó él, no el sujeto.
pub(crate) const THE_DRIVER_CRASH: &str = "uncaught";

/// Lo que se anota cuando al conductor se le acaba la paciencia sin que nadie responda.
pub(crate) const THE_EXHAUSTED_PATIENCE: &str = "timeout";

/// Lo que se pudo medir de un trámite: si el sujeto llegó a arrancar, el código SAF que emitió
/// y la clase con la que el cliente publicado lo envolvió.
pub(crate) struct ErrandOutcome {
    pub(crate) launched: bool,
    pub(crate) error_type: Option<String>,
    pub(crate) error_code: Option<String>,
}

impl Probe {
    /// Corre `script` en `mode` contra el sujeto declarado, transcribiendo cada evento del
    /// cliente publicado a medida que llega, y devuelve lo que se pudo medir del trámite.
    pub(crate) fn run_errand(
        &self,
        transcript_name: &str,
        script: &str,
        mode: &str,
    ) -> ErrandOutcome {
        let trust_root = the_trust_root_as_pem(&self.trust_root);
        let mut driver =
            the_published_client_running(trust_root.path(), self.patience, script, mode);
        let events = driver.stdout.take().expect("el conductor escribe eventos");
        let mut transcript =
            Transcript::open(&self.dossier, transcript_name).unwrap_or_else(|complaint| {
                eprintln!("{complaint}");
                std::process::exit(1);
            });
        let mut subject = None;
        let mut error_type = None;
        let mut error_code = None;
        for event in BufReader::new(events).lines().map_while(Result::ok) {
            println!("{event}");
            let _ = std::io::stdout().flush();
            transcript.record(&event).unwrap_or_else(|complaint| {
                eprintln!("{complaint}");
                std::process::exit(1);
            });
            if let Some(url) = the_launch_url_in(&event) {
                eprintln!("sondeo: invoco {} con {url}", self.subject.display());
                subject = Some(the_subject_invoked_with(&self.subject, &url));
            }
            if let Some(kind) = the_error_type_in(&event) {
                error_type = Some(kind);
            }
            if let Some(code) = the_saf_code_in(&event) {
                error_code = Some(code);
            }
            if event.contains("\"event\":\"timeout\"") {
                error_type = Some(THE_EXHAUSTED_PATIENCE.to_owned());
            }
        }
        let launched = subject.is_some();
        let _ = driver.wait();
        if let Some(mut subject) = subject {
            let _ = subject.kill();
            let _ = subject.wait();
        }
        ErrandOutcome {
            launched,
            error_type,
            error_code,
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

fn the_subject_invoked_with(subject: &Path, url: &str) -> Child {
    Command::new(subject)
        .arg(url)
        .spawn()
        .unwrap_or_else(|error| panic!("{} no arrancó: {error}", subject.display()))
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
}
