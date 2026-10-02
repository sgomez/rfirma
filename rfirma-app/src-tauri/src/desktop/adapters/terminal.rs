//! La entrada de la línea de órdenes: compone sus puertos y escribe en stdout y stderr lo que deja el caso de uso, sin Tauri ni ventana.

use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use crate::desktop::adapters::handover::SpawnedDesktop;
use crate::desktop::adapters::paths::Paths;
use crate::desktop::application::command_line::{attend, CommandLinePorts, FAILED};
use crate::desktop::ports::{CertificateStores, Terminal};
use crate::identity::adapters::pkcs11::stores::discovered_module_named;
use crate::identity::adapters::{desktop_stores, DesktopToken};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::store::Store;
use crate::identity::every_store;
use crate::identity::ports::Token;

/// Los almacenes que se recorren con el token de esta plataforma.
pub struct SeenStores {
    stores: Vec<Store>,
}

impl SeenStores {
    /// Exactamente estos almacenes.
    pub fn over(stores: Vec<Store>) -> Self {
        Self { stores }
    }

    /// Los almacenes de esta máquina, con el Almacén de rFirma si se sabe dónde vive.
    pub fn of_this_machine() -> Self {
        let configured = desktop_stores();
        match Paths::from_environment() {
            Ok(paths) => Self::over(every_store(configured, &paths.installed_certificates_dir())),
            Err(_) => Self::over(configured),
        }
    }
}

impl CertificateStores for SeenStores {
    fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        DesktopToken.list_across(&self.stores)
    }

    fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        discovered_module_named(&self.stores, library)
    }
}

/// La terminal del proceso: hay alguien al otro lado si la entrada estándar es una TTY.
pub struct ProcessTerminal;

impl Terminal for ProcessTerminal {
    fn is_interactive(&self) -> bool {
        std::io::stdin().is_terminal()
    }
}

/// Atiende la línea de órdenes de este argv, con el ejecutable delante, y devuelve el código de salida.
pub fn run_the_command_line(argv: &[String]) -> i32 {
    let ports = CommandLinePorts {
        stores: &SeenStores::of_this_machine(),
        terminal: &ProcessTerminal,
        desktop: &SpawnedDesktop,
    };
    let outcome = attend(argv.get(1..).unwrap_or_default(), &ports);
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
