//! La línea de órdenes del sondeo: su uso, las coordenadas de la tanda, las preguntas por
//! teclado y el arranque que denuncia lo que falta antes de medir nada.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::dossier::HeaderCoordinates;
use crate::errand::the_published_client;
use crate::{CaseCommand, Probe};

pub(crate) const USAGE: &str = "\
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

/// Pregunta `label` por teclado y devuelve lo escrito, sin el salto de línea final.
pub(crate) fn ask(label: &str) -> String {
    print!("{label}: ");
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
