//! Suite de conformidad: el cliente publicado bajo Node corre un guion del banco contra el
//! binario declarado y transcribe lo que viajó, sin mirar el interior del sujeto.

mod cases;
mod cli;
mod dossier;
mod errand;
mod monitor;
mod protocol;
mod transcript;
mod verdicts;

use std::path::PathBuf;
use std::time::Duration;

use dossier::Dossier;

struct Probe {
    subject: PathBuf,
    trust_root: PathBuf,
    dossier: PathBuf,
    patience: Duration,
    command: CaseCommand,
    coordinates: cli::PartialCoordinates,
    verbose: bool,
    monitor: monitor::ProgressMonitor,
}

enum CaseCommand {
    List,
    Run { case: String, relaunch: bool },
    RunPending,
    RunProtocol,
}

fn main() {
    match Probe::from_the_command_line() {
        Ok(probe) => probe.run(),
        Err(complaint) => {
            eprintln!("{complaint}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
    }
}

impl Probe {
    pub(crate) fn subject_log_path(&self) -> PathBuf {
        if let Some(parent) = self.dossier.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.join("probe-subject.log");
            }
        }
        PathBuf::from(".scratch/probe-subject.log")
    }

    fn run(mut self) {
        if let Err(complaints) = cli::preflight(&self.subject, &self.trust_root) {
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
            cases::KNOWN_CASES,
            protocol::KNOWN_CONDITIONS,
            header_coordinates,
        )
        .unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
        match &self.command {
            CaseCommand::List => verdicts::list(&dossier),
            CaseCommand::Run { case, relaunch } => {
                self.run_one(&mut dossier, case, *relaunch);
            }
            CaseCommand::RunPending => self.run_pending(&mut dossier),
            CaseCommand::RunProtocol => self.run_protocol_lane(&mut dossier),
        }
    }
}
