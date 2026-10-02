//! La entrada de la línea de órdenes: escribe en stdout y stderr lo que deja el caso de uso, sin Tauri ni ventana.

use std::io::Write;

use crate::desktop::adapters::handover::SpawnedDesktop;
use crate::desktop::application::command_line::{attend, FAILED};

/// Atiende la línea de órdenes de este argv, con el ejecutable delante, y devuelve el código de salida.
pub fn run_the_command_line(argv: &[String]) -> i32 {
    let outcome = attend(argv.get(1..).unwrap_or_default(), &SpawnedDesktop);
    for line in &outcome.stderr {
        eprintln!("{line}");
    }
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = stdout
        .write_all(&outcome.stdout)
        .and_then(|()| stdout.flush())
    {
        eprintln!("rfirma: no se puede escribir en la salida estándar ({error})");
        return FAILED;
    }
    outcome.exit_code
}
