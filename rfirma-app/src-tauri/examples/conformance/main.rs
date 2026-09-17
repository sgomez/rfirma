//! Suite de conformidad: el cliente publicado bajo Node corre un guion del banco contra el
//! binario declarado y transcribe lo que viajó, sin mirar el interior del sujeto.

mod catalogue;
mod checks;
mod cli;
mod dossier;
mod errand;
mod monitor;
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
    command: Command,
    coordinates: cli::PartialCoordinates,
    verbose: bool,
    monitor: monitor::ProgressMonitor,
}

enum Command {
    List { suite: Option<String> },
    Run { check: String, relaunch: bool },
    RunPending { suite: Option<String> },
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
        let catalogue = catalogue::read_the_catalogue().unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
        if let Err(complaint) = cli::the_suite_asked_for(&self.command, &catalogue) {
            eprintln!("{complaint}");
            std::process::exit(2);
        }
        let header_coordinates = if self.dossier.exists() {
            None
        } else {
            Some(self.coordinates_for_a_new_dossier())
        };
        let mut dossier = Dossier::open(
            &self.dossier,
            &self.subject.display().to_string(),
            &catalogue,
            header_coordinates,
        )
        .unwrap_or_else(|complaint| {
            eprintln!("{complaint}");
            std::process::exit(1);
        });
        match &self.command {
            Command::List { suite } => verdicts::list(&dossier, &catalogue, suite.as_deref()),
            Command::Run { check, relaunch } => {
                self.run_one(&mut dossier, &catalogue, check, *relaunch);
            }
            Command::RunPending { suite } => {
                self.run_pending(&mut dossier, &catalogue, suite.as_deref());
            }
        }
    }
}
