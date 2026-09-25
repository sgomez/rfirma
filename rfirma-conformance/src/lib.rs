//! Suite de conformidad: el cliente publicado bajo Node corre un guion del banco contra el
//! binario declarado y transcribe lo que viajó, y una consola web local la lanza y la sigue.

pub mod catalogue;
mod checks;
mod client;
mod comparison;
pub mod console;
mod errand;
mod harness;
mod judge;
pub mod known_bug;
pub mod label;
mod livelog;
mod manifest;
mod matrix;
mod outcome;
mod report;
mod report_view;
pub mod saf_table;
pub mod server;
mod snapshot;
mod transcript;
mod validation;
mod witness;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Lo que necesita una pasada para medir: el cliente aislado, la raíz con la que sirve, el informe
/// donde se transcribe y quien corre los trámites.
struct Probe {
    client: PathBuf,
    trust_root: PathBuf,
    report: PathBuf,
    patience: Duration,
    witness: Arc<dyn witness::Witness>,
    runner: Arc<dyn errand::ErrandRunner>,
}
