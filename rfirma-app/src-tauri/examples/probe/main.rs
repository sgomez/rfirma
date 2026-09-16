//! Sondeo: el cliente publicado bajo Node corre un guion del banco contra el binario declarado
//! y transcribe lo que viajó, sin mirar el interior del sujeto.

mod dossier;
mod transcript;

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use dossier::{CaseState, Dossier, HeaderCoordinates};
use transcript::Transcript;

const USAGE: &str = "\
uso: cargo run --example probe -- --subject <binario> --trust-root <certificado> \
--dossier <ruta> <orden>

  --subject          el binario que recibe la URL de arranque, como haría el escritorio al
                      resolver el esquema afirma://
  --trust-root       la raíz de confianza con la que ese binario sirve el canal, en PEM o en DER
  --dossier          dónde vive el expediente de la tanda; se crea si no existe
  --patience-ms      cuánto espera el conductor antes de darse por vencido (por omisión, 60000)

  Coordenadas de la tanda, obligatorias solo al abrir un expediente nuevo:
  --os               sistema operativo del sujeto
  --os-version       su versión
  --subject-version  versión del sujeto sondeado
  --transport        transporte del canal (sockets, servidor intermedio, ...)
  --store            almacén de certificados con el que se sondeó

órdenes:
  list                lista los casos del expediente con su estado y su fecha
  run <caso>          ejecuta un caso por su nombre; si ya está resuelto, no repite salvo --relaunch
  run-pending         ejecuta, por orden, los casos que sigan pendientes; se detiene si uno falla
";

const DEFAULT_PATIENCE: Duration = Duration::from_millis(60_000);

/// El guion de una sola selección, el más corto que hace saludar al cliente publicado.
const THE_SINGLE_SELECTION: &str = "selectcert";

/// El guion de `sign` con `format=CAdES` y `mode=explicit`: un trámite entero, de punta a punta.
const THE_FULL_ERRAND: &str = "signcades";

/// El modo que el cliente publicado habla de por sí: puertos sorteados y `v=4`.
const THE_FOURTH_PROTOCOL: &str = "v4";

/// Los nombres de los casos que el sondeo sabe ejecutar.
const KNOWN_CASES: &[&str] = &["saludo", "tramite"];

struct Probe {
    subject: PathBuf,
    trust_root: PathBuf,
    dossier: PathBuf,
    patience: Duration,
    command: CaseCommand,
    coordinates: PartialCoordinates,
}

/// Las coordenadas de la tanda, según llegaron de la línea de órdenes: puede que falte alguna.
#[derive(Default)]
struct PartialCoordinates {
    os: Option<String>,
    os_version: Option<String>,
    subject_version: Option<String>,
    transport: Option<String>,
    store: Option<String>,
}

impl PartialCoordinates {
    /// Las coordenadas completas, o el nombre de cada flag que falta.
    fn complete(self) -> Result<HeaderCoordinates, Vec<&'static str>> {
        let mut missing = Vec::new();
        if self.os.is_none() {
            missing.push("--os");
        }
        if self.os_version.is_none() {
            missing.push("--os-version");
        }
        if self.subject_version.is_none() {
            missing.push("--subject-version");
        }
        if self.transport.is_none() {
            missing.push("--transport");
        }
        if self.store.is_none() {
            missing.push("--store");
        }
        if !missing.is_empty() {
            return Err(missing);
        }
        Ok(HeaderCoordinates {
            os: self.os.unwrap(),
            os_version: self.os_version.unwrap(),
            subject_version: self.subject_version.unwrap(),
            transport: self.transport.unwrap(),
            store: self.store.unwrap(),
        })
    }
}

enum CaseCommand {
    List,
    Run { case: String, relaunch: bool },
    RunPending,
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
        let mut dossier = None;
        let mut patience = DEFAULT_PATIENCE;
        let mut coordinates = PartialCoordinates::default();
        let mut arguments = std::env::args().skip(1);
        let command = loop {
            let flag = arguments
                .next()
                .ok_or_else(|| "falta la orden: list, run <caso> o run-pending".to_owned())?;
            match flag.as_str() {
                "--subject" => subject = Some(PathBuf::from(value_of(&flag, &mut arguments)?)),
                "--trust-root" => {
                    trust_root = Some(PathBuf::from(value_of(&flag, &mut arguments)?));
                }
                "--dossier" => dossier = Some(PathBuf::from(value_of(&flag, &mut arguments)?)),
                "--patience-ms" => {
                    let milliseconds = value_of(&flag, &mut arguments)?
                        .parse()
                        .map_err(|_| "--patience-ms quiere un número de milisegundos".to_owned())?;
                    patience = Duration::from_millis(milliseconds);
                }
                "--os" => coordinates.os = Some(value_of(&flag, &mut arguments)?),
                "--os-version" => coordinates.os_version = Some(value_of(&flag, &mut arguments)?),
                "--subject-version" => {
                    coordinates.subject_version = Some(value_of(&flag, &mut arguments)?);
                }
                "--transport" => coordinates.transport = Some(value_of(&flag, &mut arguments)?),
                "--store" => coordinates.store = Some(value_of(&flag, &mut arguments)?),
                "list" => break CaseCommand::List,
                "run-pending" => break CaseCommand::RunPending,
                "run" => {
                    let case = value_of(&flag, &mut arguments)?;
                    let relaunch = arguments.next().as_deref() == Some("--relaunch");
                    break CaseCommand::Run { case, relaunch };
                }
                other => return Err(format!("argumento desconocido: {other}")),
            }
        };
        Ok(Self {
            subject: subject.ok_or_else(|| "falta --subject".to_owned())?,
            trust_root: trust_root.ok_or_else(|| "falta --trust-root".to_owned())?,
            dossier: dossier.ok_or_else(|| "falta --dossier".to_owned())?,
            patience,
            command,
            coordinates,
        })
    }

    fn run(mut self) {
        if let Err(complaints) = preflight(&self.subject, &self.trust_root) {
            for complaint in &complaints {
                eprintln!("{complaint}");
            }
            eprintln!("\nun fallo de condición no es un veredicto: no se ha llegado a medir nada.");
            std::process::exit(3);
        }
        let header_coordinates = if self.dossier.exists() {
            None
        } else {
            match std::mem::take(&mut self.coordinates).complete() {
                Ok(coordinates) => Some(coordinates),
                Err(missing) => {
                    eprintln!("faltan las coordenadas de la tanda: {}", missing.join(", "));
                    std::process::exit(3);
                }
            }
        };
        let mut dossier = Dossier::open(
            &self.dossier,
            &self.subject.display().to_string(),
            KNOWN_CASES,
            header_coordinates,
        )
        .unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
        match &self.command {
            CaseCommand::List => list(&dossier),
            CaseCommand::Run { case, relaunch } => {
                self.run_one(&mut dossier, case, *relaunch);
            }
            CaseCommand::RunPending => self.run_pending(&mut dossier),
        }
    }

    fn run_one(&self, dossier: &mut Dossier, case: &str, relaunch: bool) {
        if !KNOWN_CASES.contains(&case) {
            eprintln!(
                "no conozco el caso «{case}»; los que hay son: {}",
                KNOWN_CASES.join(", ")
            );
            std::process::exit(2);
        }
        if !relaunch && dossier.state_of(case) == Some(CaseState::Resolved) {
            println!("el caso «{case}» ya está resuelto; usa --relaunch para repetirlo");
            return;
        }
        self.run_case(case);
        dossier.resolve(case).unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
    }

    fn run_pending(&self, dossier: &mut Dossier) {
        for case in KNOWN_CASES {
            if dossier.state_of(case) == Some(CaseState::Resolved) {
                continue;
            }
            self.run_case(case);
            dossier.resolve(case).unwrap_or_else(|complaint| {
                eprintln!("{complaint}");
                std::process::exit(1);
            });
        }
    }

    fn run_case(&self, case: &str) {
        match case {
            "saludo" => self.run_errand(case, THE_SINGLE_SELECTION),
            "tramite" => self.run_errand(case, THE_FULL_ERRAND),
            other => unreachable!("caso sin arnés: {other}"),
        }
    }

    /// Corre `script` de punta a punta contra el sujeto declarado, transcribiendo cada evento
    /// del cliente publicado a medida que llega.
    fn run_errand(&self, case: &str, script: &str) {
        let trust_root = the_trust_root_as_pem(&self.trust_root);
        let mut driver = the_published_client_running(trust_root.path(), self.patience, script);
        let events = driver.stdout.take().expect("el conductor escribe eventos");
        let mut transcript = Transcript::open(&self.dossier, case).unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
        let mut subject = None;
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
        }
        let _ = driver.wait();
        if let Some(mut subject) = subject {
            let _ = subject.kill();
            let _ = subject.wait();
        }
    }
}

fn list(dossier: &Dossier) {
    let header = dossier.header();
    println!(
        "tanda del {}: {} {}, sujeto {}, transporte {}, almacén {}",
        header.date,
        header.os,
        header.os_version,
        header.subject_version,
        header.transport,
        header.store
    );
    for (case, record) in dossier.cases() {
        let state = match record.state {
            CaseState::Pending => "pendiente",
            CaseState::Resolved => "resuelto",
        };
        let date = record.date.as_deref().unwrap_or("-");
        println!("{case}\t{state}\t{date}");
    }
}

/// Antes de intentar nada: comprueba que hay con qué sondear. Un fallo aquí no es un veredicto,
/// es que no se ha llegado a medir.
fn preflight(subject: &Path, trust_root: &Path) -> Result<(), Vec<String>> {
    let mut complaints = Vec::new();
    if Command::new("node").arg("--version").output().is_err() {
        complaints.push("falta Node en el PATH".to_owned());
    }
    let published_client = the_published_client();
    if !published_client.exists() {
        complaints.push(format!(
            "falta {}: ejecuta `just autoscript`",
            published_client.display()
        ));
    }
    if !is_executable(subject) {
        complaints.push(format!(
            "el sujeto {} no existe o no se puede ejecutar",
            subject.display()
        ));
    }
    if !trust_root.exists() {
        complaints.push(format!(
            "la raíz de confianza {} no está donde se dijo",
            trust_root.display()
        ));
    }
    if complaints.is_empty() {
        Ok(())
    } else {
        Err(complaints)
    }
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
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

fn the_published_client_running(trust_root: &Path, patience: Duration, script: &str) -> Child {
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
