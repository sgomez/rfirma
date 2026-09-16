//! Sondeo del saludo: el cliente publicado bajo Node invoca al binario declarado y el eco vuelve.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const USAGE: &str = "\
uso: cargo run --example probe -- --subject <binario> --trust-root <certificado> [--patience-ms <ms>]

  --subject      el binario que recibe la URL de arranque, como haría el escritorio al resolver
                 el esquema afirma://
  --trust-root   la raíz de confianza con la que ese binario sirve el canal, en PEM o en DER
  --patience-ms  cuánto espera el conductor antes de darse por vencido (por omisión, 60000)
";

const DEFAULT_PATIENCE: Duration = Duration::from_millis(60_000);

/// El guion de una sola selección, el más corto que hace saludar al cliente publicado.
const THE_SINGLE_SELECTION: &str = "selectcert";

/// El modo que el cliente publicado habla de por sí: puertos sorteados y `v=4`.
const THE_FOURTH_PROTOCOL: &str = "v4";

struct Probe {
    subject: PathBuf,
    trust_root: PathBuf,
    patience: Duration,
}

fn main() {
    match Probe::from_the_command_line() {
        Ok(probe) => probe.run(),
        Err(complaint) => {
            eprintln!("{complaint}\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}

impl Probe {
    fn from_the_command_line() -> Result<Self, String> {
        let mut subject = None;
        let mut trust_root = None;
        let mut patience = DEFAULT_PATIENCE;
        let mut arguments = std::env::args().skip(1);
        while let Some(flag) = arguments.next() {
            match flag.as_str() {
                "--subject" => subject = Some(PathBuf::from(value_of(&flag, &mut arguments)?)),
                "--trust-root" => {
                    trust_root = Some(PathBuf::from(value_of(&flag, &mut arguments)?));
                }
                "--patience-ms" => {
                    let milliseconds = value_of(&flag, &mut arguments)?
                        .parse()
                        .map_err(|_| "--patience-ms quiere un número de milisegundos".to_owned())?;
                    patience = Duration::from_millis(milliseconds);
                }
                other => return Err(format!("argumento desconocido: {other}")),
            }
        }
        Ok(Self {
            subject: subject.ok_or_else(|| "falta --subject".to_owned())?,
            trust_root: trust_root.ok_or_else(|| "falta --trust-root".to_owned())?,
            patience,
        })
    }

    fn run(self) {
        let trust_root = the_trust_root_as_pem(&self.trust_root);
        let mut driver = the_published_client_running(trust_root.path(), self.patience);
        let events = driver.stdout.take().expect("el conductor escribe eventos");
        let mut subject = None;
        for event in BufReader::new(events).lines().map_while(Result::ok) {
            println!("{event}");
            let _ = std::io::stdout().flush();
            if let Some(url) = the_launch_url_in(&event) {
                eprintln!("sondeo: invoco {} con {url}", self.subject.display());
                subject = Some(the_subject_invoked_with(&self.subject, &url));
            }
        }
        let _ = driver.wait();
        if let Some(mut subject) = subject {
            let _ = subject.kill();
            let _ = subject.wait();
        }
    }
}

fn value_of(flag: &str, arguments: &mut impl Iterator<Item = String>) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{flag} se quedó sin valor"))
}

/// El `autoscript.js` del tag `v1.9.2`, donde lo deja `just autoscript`.
fn the_published_client() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/conformance/autoscript-1.9.2.js")
}

/// El conductor de Node que le monta el navegador mínimo alrededor.
fn the_driver() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/driver.mjs")
}

fn the_published_client_running(trust_root: &Path, patience: Duration) -> Child {
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
        .env("RFIRMA_BENCH_MODE", THE_FOURTH_PROTOCOL)
        .env("RFIRMA_BENCH_SCRIPT", THE_SINGLE_SELECTION)
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
