//! La línea de órdenes de la suite de conformidad: su uso, las coordenadas de la tanda, las
//! preguntas por teclado y el arranque que denuncia lo que falta antes de medir nada.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::catalogue::{Check, THE_SUITES};
use crate::dossier::HeaderCoordinates;
use crate::errand::the_published_client;
use crate::{Command, Probe};

pub(crate) const USAGE: &str = "\
uso: cargo run --example conformance -- --subject <binario> --trust-root <certificado> \
--dossier <ruta> <orden>

  --subject          el binario que recibe la URL de arranque, como haría el escritorio al
                      resolver el esquema afirma://
  --trust-root       la raíz de confianza con la que ese binario sirve el canal, en PEM o en DER
  --dossier          dónde vive el expediente de la tanda; se crea si no existe
  --patience-ms      cuánto espera el conductor antes de darse por vencido (por omisión, 60000)
  --plain            fuerza el modo lineal limpio sin secuencias ANSI ni sobreescritura
  --verbose          emite las tramas JSON de WebSocket y trazas de depuración a la consola

  Coordenadas de la tanda, obligatorias solo al abrir un expediente nuevo:
  --os               sistema operativo del sujeto
  --os-version       su versión
  --subject-version  versión del sujeto sondeado; si falta y la entrada es un terminal, se
                      pregunta por teclado una sola vez
  --transport        transporte del canal; por omisión, websocket (el único de esta fase)
  --store            almacén de certificados con el que se sondeó

órdenes:
  list                lista las comprobaciones del expediente con su estado y su fecha
  run <id>            ejecuta una comprobación por su identificador; si ya está resuelta, no
                      repite salvo --relaunch
  run-pending         ejecuta, por orden del catálogo, las que sigan pendientes

  --suite <conjunto>  acota `list` y `run-pending` a un conjunto del catálogo
";

const DEFAULT_PATIENCE: Duration = Duration::from_millis(60_000);

/// Las coordenadas de la tanda, según llegaron de la línea de órdenes: puede que falte alguna.
#[derive(Default)]
pub(crate) struct PartialCoordinates {
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

/// Los flags que pueden venir detrás de la orden, que son los mismos que pueden venir delante.
fn read_the_trailing_flags(
    arguments: &mut impl Iterator<Item = String>,
    plain: &mut bool,
    verbose: &mut bool,
    suite: &mut Option<String>,
    relaunch: &mut bool,
) -> Result<(), String> {
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--plain" => *plain = true,
            "--verbose" => *verbose = true,
            "--relaunch" => *relaunch = true,
            "--suite" => *suite = Some(value_of("--suite", arguments)?),
            other => return Err(format!("argumento desconocido: {other}")),
        }
    }
    Ok(())
}

/// El flag que la orden no usa, denunciado en vez de tragado en silencio.
fn no_flag_the_command_ignores(command: &str, ignored: &[(&str, bool)]) -> Result<(), String> {
    for (flag, given) in ignored {
        if *given {
            return Err(format!("«{command}» no acepta {flag}"));
        }
    }
    Ok(())
}

/// El conjunto que la orden acota, si está en el vocabulario y si alguna entrada lo declara.
pub(crate) fn the_suite_asked_for(command: &Command, catalogue: &[Check]) -> Result<(), String> {
    let asked = match command {
        Command::List { suite } | Command::RunPending { suite } => suite.as_deref(),
        Command::Run { .. } => None,
    };
    let Some(wanted) = asked else {
        return Ok(());
    };
    if !THE_SUITES.contains(&wanted) {
        return Err(format!(
            "no conozco el conjunto «{wanted}»; los que hay son: {}",
            THE_SUITES.join(", ")
        ));
    }
    if !catalogue.iter().any(|check| check.suite == wanted) {
        return Err(format!(
            "el conjunto «{wanted}» no tiene ninguna comprobación en el catálogo"
        ));
    }
    Ok(())
}

/// Pregunta `label` por teclado y devuelve lo escrito, sin el salto de línea final.
pub(crate) fn ask(label: &str) -> String {
    let is_tty = std::io::stdout().is_terminal() && std::io::stdin().is_terminal();
    let box_str = crate::monitor::render_dialog_box(label, is_tty);
    println!("\n{box_str}");
    print!("> ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .expect("no pude leer de teclado");
    line.trim().to_owned()
}

impl Probe {
    pub(crate) fn from_the_command_line() -> Result<Self, String> {
        let mut subject = None;
        let mut trust_root = None;
        let mut dossier = None;
        let mut patience = DEFAULT_PATIENCE;
        let mut coordinates = PartialCoordinates::default();
        let mut plain = false;
        let mut verbose = false;
        let mut suite = None;
        let mut relaunch = false;
        let mut arguments = std::env::args().skip(1);
        let command = loop {
            let flag = arguments
                .next()
                .ok_or_else(|| "falta la orden: list, run <id> o run-pending".to_owned())?;
            match flag.as_str() {
                "--suite" => suite = Some(value_of(&flag, &mut arguments)?),
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
                "--plain" => plain = true,
                "--verbose" => verbose = true,
                "--os" => coordinates.os = Some(value_of(&flag, &mut arguments)?),
                "--os-version" => coordinates.os_version = Some(value_of(&flag, &mut arguments)?),
                "--subject-version" => {
                    coordinates.subject_version = Some(value_of(&flag, &mut arguments)?);
                }
                "--transport" => coordinates.transport = Some(value_of(&flag, &mut arguments)?),
                "--store" => coordinates.store = Some(value_of(&flag, &mut arguments)?),
                "list" => {
                    read_the_trailing_flags(
                        &mut arguments,
                        &mut plain,
                        &mut verbose,
                        &mut suite,
                        &mut relaunch,
                    )?;
                    no_flag_the_command_ignores("list", &[("--relaunch", relaunch)])?;
                    break Command::List {
                        suite: suite.take(),
                    };
                }
                "run-pending" => {
                    read_the_trailing_flags(
                        &mut arguments,
                        &mut plain,
                        &mut verbose,
                        &mut suite,
                        &mut relaunch,
                    )?;
                    no_flag_the_command_ignores("run-pending", &[("--relaunch", relaunch)])?;
                    break Command::RunPending {
                        suite: suite.take(),
                    };
                }
                "run" => {
                    let check = value_of(&flag, &mut arguments)?;
                    read_the_trailing_flags(
                        &mut arguments,
                        &mut plain,
                        &mut verbose,
                        &mut suite,
                        &mut relaunch,
                    )?;
                    no_flag_the_command_ignores("run", &[("--suite", suite.is_some())])?;
                    break Command::Run { check, relaunch };
                }
                other => return Err(format!("argumento desconocido: {other}")),
            }
        };
        let monitor = crate::monitor::ProgressMonitor::new(plain);
        Ok(Self {
            subject: subject.ok_or_else(|| "falta --subject".to_owned())?,
            trust_root: trust_root.ok_or_else(|| "falta --trust-root".to_owned())?,
            dossier: dossier.ok_or_else(|| "falta --dossier".to_owned())?,
            patience,
            command,
            coordinates,
            verbose,
            monitor,
        })
    }

    /// Las coordenadas para abrir un expediente nuevo: la versión del sujeto se pregunta por
    /// teclado si falta y hay alguien delante; el resto, si falta, aborta nombrando el flag.
    pub(crate) fn coordinates_for_a_new_dossier(&mut self) -> HeaderCoordinates {
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
}

/// Antes de intentar nada: comprueba que hay con qué sondear. Un fallo aquí no es un veredicto,
/// es que no se ha llegado a medir.
pub(crate) fn preflight(subject: &Path, trust_root: &Path) -> Result<(), Vec<String>> {
    let mut complaints = Vec::new();
    if std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_err()
    {
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
