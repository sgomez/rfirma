//! Suite de conformidad: el cliente publicado bajo Node corre un guion del banco contra el
//! binario declarado y transcribe lo que viajó, y una consola web local la lanza y la sigue.

mod catalogue;
mod checks;
mod client;
mod comparison;
mod console;
mod errand;
mod livelog;
mod outcome;
mod report;
mod report_view;
mod server;
mod snapshot;
mod transcript;
mod validation;
mod verdicts;

use std::path::PathBuf;
use std::time::Duration;

const USAGE: &str = "\
uso: just conformance

Levanta la consola web de la suite, imprime su URL con el token y la abre en el navegador. El
cliente, el informe y las comprobaciones se eligen en la página; no hay órdenes ni banderas.
";

/// Cuánto espera el conductor a que alguien responda si la comprobación no declara otra cosa.
const THE_PATIENCE: Duration = Duration::from_millis(60_000);

/// Lo que necesita una pasada para medir: el cliente aislado, la raíz con la que sirve y el informe
/// donde se transcribe.
struct Probe {
    client: PathBuf,
    trust_root: PathBuf,
    report: PathBuf,
    patience: Duration,
    witness: console::Witness,
}

fn main() {
    if std::env::args().len() > 1 {
        eprint!("{USAGE}");
        std::process::exit(2);
    }
    let catalogue = catalogue::read_the_catalogue().unwrap_or_else(|complaint| {
        eprintln!("{complaint}");
        std::process::exit(1);
    });
    let server = server::Server::bind().unwrap_or_else(|complaint| {
        eprintln!("{complaint}");
        std::process::exit(1);
    });
    let console = console::Console::new(catalogue, the_reports_dir(), THE_PATIENCE);
    console.start_the_runner();
    let url = server.url();
    println!("consola de conformidad en {url}");
    if std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .is_err()
    {
        println!("ábrela en el navegador: no he podido hacerlo yo");
    }
    server.serve(&console);
}

fn the_reports_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../reports/conformance")
}
