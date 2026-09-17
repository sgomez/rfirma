//! Sondeo: el cliente publicado bajo Node corre un guion del banco contra el binario declarado
//! y transcribe lo que viajó, sin mirar el interior del sujeto.

mod dossier;
mod transcript;

use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use dossier::{CaseState, Dossier, HeaderCoordinates, Verdict};
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
  --subject-version  versión del sujeto sondeado; si falta y la entrada es un terminal, se
                      pregunta por teclado una sola vez
  --transport        transporte del canal; por omisión, websocket (el único de esta fase)
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

/// El caso que mide BUG-25: si el canal rechaza igual una versión obsoleta que una no
/// soportada, en vez de distinguirlas.
const THE_PROTOCOL_FRESHNESS_CASE: &str = "obsolete_and_unsupported_protocol_share_error_code";

/// El guion de `save`, el que dispara la ventana nativa de destino: necesita a una persona
/// delante para completarse, así que no cabe entre los casos automáticos.
const THE_SAVE_SCRIPT: &str = "save";

/// El caso interactivo que mide BUG-18: si el guardado por WebSocket pide de verdad un destino.
const THE_SAVE_DESTINATION_CASE: &str = "save_over_websocket_asks_for_a_destination";

/// El modo del conductor que hace hablar al cliente publicado con el bucle local IPv6.
const THE_IPV6_LOOPBACK_MODE: &str = "v4-ipv6";

/// El caso que mide BUG-11: si el canal de la versión 4 sigue rechazando el bucle local IPv6.
const THE_IPV6_LOOPBACK_CASE: &str = "ipv6_loopback_is_rejected_on_the_v4_channel";

/// El código con el que el canal de la versión 4 rechaza una procedencia que no es exactamente
/// `127.0.0.1`.
const SAF_47_EXTERNAL_REQUEST: &str = "SAF_47";

/// El modo del conductor que fuerza el transporte `service` a hablar por una lista fija de
/// puertos, la misma que el caso ocupa de antemano.
const THE_SOCKET_BIND_FAILURE_MODE: &str = "service-bind-failure";

/// El caso que mide BUG-10: si un fallo al ligar el socket sigue siendo indistinguible, para el
/// cliente publicado, de que la aplicación no está instalada.
const THE_SOCKET_BIND_FAILURE_CASE: &str =
    "an_occupied_socket_makes_the_client_report_the_app_as_missing";

/// El error con el que el cliente publicado se rinde tras agotar los reintentos de conexión: el
/// mismo que arroja cuando la aplicación no está instalada.
const APPLICATION_NOT_FOUND_EXCEPTION: &str =
    "es.gob.afirma.standalone.ApplicationNotFoundException";

/// El `type` con el que el conductor avisa de que reventó él, no el sujeto.
const THE_DRIVER_CRASH: &str = "uncaught";

/// Los puertos fijos que fuerza `THE_SOCKET_BIND_FAILURE_MODE`, y que el caso ocupa antes de
/// invocar al sujeto para que no le quede ninguno libre.
const THE_SOCKET_BIND_FAILURE_PORTS: [u16; 3] = [63131, 63132, 63133];

/// El código con el que `signandsave` debería rechazar la falta del verbo (`cop`), si validara
/// su presencia como hace `sign`.
const THE_SIGN_AND_SAVE_SCRIPT: &str = "signandsavewithoutaverb";

/// El caso que mide BUG-15: qué código de error llega de verdad al cliente publicado cuando
/// `signandsave` no recibe verbo.
const THE_VERB_VALIDATION_CASE: &str = "signandsave_without_a_verb_reports_its_real_error_code";

/// El código engañoso con el que BUG-15 documenta que se reporta la falta de verbo.
const SAF_09_MISLEADING_ERROR: &str = "SAF_09";

/// Los nombres de los casos que el sondeo sabe ejecutar.
const KNOWN_CASES: &[&str] = &[
    "saludo",
    "tramite",
    THE_PROTOCOL_FRESHNESS_CASE,
    THE_SAVE_DESTINATION_CASE,
    THE_IPV6_LOOPBACK_CASE,
    THE_SOCKET_BIND_FAILURE_CASE,
    THE_VERB_VALIDATION_CASE,
];

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

/// El único transporte que habla esta fase; el relé por servidor intermedio no está sondeado
/// todavía.
const DEFAULT_TRANSPORT: &str = "websocket";

impl PartialCoordinates {
    /// Las coordenadas completas, o el nombre de cada flag que falta. `--transport` nunca falta:
    /// sin dar, vale `DEFAULT_TRANSPORT`.
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
            transport: self
                .transport
                .unwrap_or_else(|| DEFAULT_TRANSPORT.to_owned()),
            store: self.store.unwrap(),
        })
    }
}

/// Pregunta `label` por teclado y devuelve lo escrito, sin el salto de línea final.
fn ask(label: &str) -> String {
    print!("{label}: ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .expect("no pude leer de teclado");
    line.trim().to_owned()
}

enum CaseCommand {
    List,
    Run { case: String, relaunch: bool },
    RunPending,
}

/// Lo que deja un caso al correr: un veredicto, con su observación si la hubo, o nada si se
/// quedó sin respuesta y hay que seguir ofreciéndolo.
enum CaseOutcome {
    Resolved {
        verdict: Verdict,
        observation: Option<String>,
    },
    StillPending,
}

impl CaseOutcome {
    fn confirmed() -> Self {
        Self::Resolved {
            verdict: Verdict::Confirmed,
            observation: None,
        }
    }

    fn resolved(verdict: Verdict) -> Self {
        Self::Resolved {
            verdict,
            observation: None,
        }
    }
}

/// Lo que se pudo medir de un trámite: si el sujeto llegó a arrancar, el código SAF que emitió
/// y la clase con la que el cliente publicado lo envolvió.
struct ErrandOutcome {
    launched: bool,
    error_type: Option<String>,
    error_code: Option<String>,
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
            Some(self.coordinates_for_a_new_dossier())
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

    /// Las coordenadas para abrir un expediente nuevo: la versión del sujeto se pregunta por
    /// teclado si falta y hay alguien delante; el resto, si falta, aborta nombrando el flag.
    fn coordinates_for_a_new_dossier(&mut self) -> HeaderCoordinates {
        if self.coordinates.subject_version.is_none() && std::io::stdin().is_terminal() {
            self.coordinates.subject_version = Some(ask("versión del sujeto sondeado"));
        }
        std::mem::take(&mut self.coordinates)
            .complete()
            .unwrap_or_else(|missing| {
                eprintln!("faltan las coordenadas de la tanda: {}", missing.join(", "));
                std::process::exit(3);
            })
    }

    fn run_one(&self, dossier: &mut Dossier, case: &str, relaunch: bool) {
        if !KNOWN_CASES.contains(&case) {
            eprintln!(
                "no conozco el caso «{case}»; los que hay son: {}",
                KNOWN_CASES.join(", ")
            );
            std::process::exit(2);
        }
        if !relaunch && matches!(dossier.state_of(case), Some(CaseState::Resolved(_))) {
            println!("el caso «{case}» ya está resuelto; usa --relaunch para repetirlo");
            return;
        }
        self.run_case_and_resolve(dossier, case);
    }

    fn run_pending(&self, dossier: &mut Dossier) {
        for case in KNOWN_CASES {
            if matches!(dossier.state_of(case), Some(CaseState::Resolved(_))) {
                continue;
            }
            self.run_case_and_resolve(dossier, case);
        }
    }

    fn run_case_and_resolve(&self, dossier: &mut Dossier, case: &str) {
        match self.run_case(case) {
            CaseOutcome::Resolved {
                verdict,
                observation,
            } => {
                dossier
                    .resolve(case, verdict, observation)
                    .unwrap_or_else(|complaint| {
                        eprintln!("{complaint}");
                        std::process::exit(1);
                    });
            }
            CaseOutcome::StillPending => {
                println!("el caso «{case}» sigue pendiente: no hubo respuesta");
            }
        }
    }

    fn run_case(&self, case: &str) -> CaseOutcome {
        match case {
            "saludo" => {
                self.run_errand(case, THE_SINGLE_SELECTION, THE_FOURTH_PROTOCOL);
                CaseOutcome::confirmed()
            }
            "tramite" => {
                self.run_errand(case, THE_FULL_ERRAND, THE_FOURTH_PROTOCOL);
                CaseOutcome::confirmed()
            }
            THE_PROTOCOL_FRESHNESS_CASE => self.run_protocol_freshness_case(),
            THE_SAVE_DESTINATION_CASE => self.run_save_destination_case(),
            THE_IPV6_LOOPBACK_CASE => self.run_ipv6_loopback_case(),
            THE_SOCKET_BIND_FAILURE_CASE => self.run_socket_bind_failure_case(),
            THE_VERB_VALIDATION_CASE => self.run_verb_validation_case(),
            other => unreachable!("caso sin arnés: {other}"),
        }
    }

    /// El código SAF que el sujeto emite al rechazar una versión obsoleta y una no soportada:
    /// si coincide, BUG-25 sigue vigente; si distingue, está corregido.
    fn run_protocol_freshness_case(&self) -> CaseOutcome {
        let obsolete = self
            .run_errand(
                &format!("{THE_PROTOCOL_FRESHNESS_CASE}-obsolete"),
                THE_SINGLE_SELECTION,
                "v1",
            )
            .error_code;
        let unsupported = self
            .run_errand(
                &format!("{THE_PROTOCOL_FRESHNESS_CASE}-unsupported"),
                THE_SINGLE_SELECTION,
                "v99",
            )
            .error_code;
        match (obsolete, unsupported) {
            (Some(obsolete), Some(unsupported)) if obsolete == unsupported => {
                CaseOutcome::Resolved {
                    verdict: Verdict::Confirmed,
                    observation: Some(obsolete),
                }
            }
            (Some(obsolete), Some(unsupported)) => CaseOutcome::Resolved {
                verdict: Verdict::Refuted,
                observation: Some(format!("{obsolete} frente a {unsupported}")),
            },
            _ => CaseOutcome::resolved(Verdict::NotObservable),
        }
    }

    /// El caso que estrena las preguntas a la persona: `save` por WebSocket dispara la ventana
    /// nativa de destino, y solo alguien delante puede confirmar que se pidió.
    fn run_save_destination_case(&self) -> CaseOutcome {
        if !std::io::stdin().is_terminal() {
            return CaseOutcome::StillPending;
        }
        println!("va a aparecer la ventana para elegir dónde guardar el fichero");
        let outcome = self.run_errand(
            THE_SAVE_DESTINATION_CASE,
            THE_SAVE_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        let answer = ask("¿se pidió elegir dónde guardar el fichero? [s/n]");
        if answer.is_empty() {
            return CaseOutcome::StillPending;
        }
        let confirmed = answer.to_lowercase().starts_with('s');
        let verdict = if confirmed && outcome.error_type.is_none() {
            Verdict::Confirmed
        } else {
            Verdict::Refuted
        };
        CaseOutcome::Resolved {
            verdict,
            observation: Some(answer),
        }
    }

    /// El código SAF con el que el canal de la versión 4 responde a una conexión por el bucle
    /// local IPv6: si es `SAF_47`, BUG-11 sigue vigente; si el trámite tira para adelante, está
    /// corregido.
    fn run_ipv6_loopback_case(&self) -> CaseOutcome {
        let outcome = self.run_errand(
            THE_IPV6_LOOPBACK_CASE,
            THE_SINGLE_SELECTION,
            THE_IPV6_LOOPBACK_MODE,
        );
        the_verdict_for_saf_code(outcome, SAF_47_EXTERNAL_REQUEST)
    }

    /// Ocupa de antemano los puertos que fuerza `THE_SOCKET_BIND_FAILURE_MODE`, para que
    /// `tryPorts` no encuentre ninguno libre, y mide si el cliente publicado acaba reportando la
    /// aplicación como no instalada en vez de un fallo de arranque.
    fn run_socket_bind_failure_case(&self) -> CaseOutcome {
        let _occupied_ports: Vec<TcpListener> = THE_SOCKET_BIND_FAILURE_PORTS
            .iter()
            .map(|port| {
                TcpListener::bind(("0.0.0.0", *port))
                    .unwrap_or_else(|error| panic!("no pude ocupar el puerto {port}: {error}"))
            })
            .collect();
        let outcome = self.run_errand(
            THE_SOCKET_BIND_FAILURE_CASE,
            THE_SINGLE_SELECTION,
            THE_SOCKET_BIND_FAILURE_MODE,
        );
        if !outcome.launched {
            return CaseOutcome::resolved(Verdict::NotObservable);
        }
        let verdict = match outcome.error_type.as_deref() {
            Some(APPLICATION_NOT_FOUND_EXCEPTION) => Verdict::Confirmed,
            Some(THE_DRIVER_CRASH) | None => Verdict::NotObservable,
            Some(_) => Verdict::Refuted,
        };
        CaseOutcome::Resolved {
            verdict,
            observation: outcome.error_code.or(outcome.error_type),
        }
    }

    /// El código con el que `signandsave` responde de verdad cuando no recibe verbo: si es el
    /// `SAF_09` engañoso que documenta BUG-15, sigue vigente.
    fn run_verb_validation_case(&self) -> CaseOutcome {
        let outcome = self.run_errand(
            THE_VERB_VALIDATION_CASE,
            THE_SIGN_AND_SAVE_SCRIPT,
            THE_FOURTH_PROTOCOL,
        );
        the_verdict_for_saf_code(outcome, SAF_09_MISLEADING_ERROR)
    }

    /// Corre `script` en `mode` contra el sujeto declarado, transcribiendo cada evento del
    /// cliente publicado a medida que llega, y devuelve lo que se pudo medir del trámite.
    fn run_errand(&self, transcript_name: &str, script: &str, mode: &str) -> ErrandOutcome {
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

/// La etiqueta en castellano de un veredicto, la que ve quien lee el listado.
fn verdict_label(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Confirmed => "confirmado",
        Verdict::Refuted => "refutado",
        Verdict::NotObservable => "no observable",
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
            CaseState::Resolved(verdict) => verdict_label(verdict),
        };
        let date = record.date.as_deref().unwrap_or("-");
        let observation = record.observation.as_deref().unwrap_or("-");
        println!("{case}\t{state}\t{date}\t{observation}");
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

/// El veredicto de una ficha que se juega a un código SAF concreto: sin código en el cable no
/// hay nada que afirmar, y el caso sale no observable en vez de refutado.
fn the_verdict_for_saf_code(outcome: ErrandOutcome, expected: &str) -> CaseOutcome {
    if !outcome.launched {
        return CaseOutcome::resolved(Verdict::NotObservable);
    }
    match outcome.error_code {
        Some(code) => CaseOutcome::Resolved {
            verdict: if code == expected {
                Verdict::Confirmed
            } else {
                Verdict::Refuted
            },
            observation: Some(code),
        },
        None => CaseOutcome::Resolved {
            verdict: Verdict::NotObservable,
            observation: outcome.error_type,
        },
    }
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

    #[test]
    fn a_missing_saf_code_is_not_observable_rather_than_refuted() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some(THE_DRIVER_CRASH.to_owned()),
            error_code: None,
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::NotObservable);
    }

    #[test]
    fn the_expected_saf_code_confirms_the_case() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some("java.lang.Exception".to_owned()),
            error_code: Some("SAF_47".to_owned()),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Confirmed);
    }

    #[test]
    fn another_saf_code_refutes_the_case() {
        let outcome = ErrandOutcome {
            launched: true,
            error_type: Some("java.lang.Exception".to_owned()),
            error_code: Some("SAF_03".to_owned()),
        };
        let CaseOutcome::Resolved { verdict, .. } = the_verdict_for_saf_code(outcome, "SAF_47")
        else {
            panic!("el caso debería resolverse");
        };
        assert_eq!(verdict, Verdict::Refuted);
    }
}
