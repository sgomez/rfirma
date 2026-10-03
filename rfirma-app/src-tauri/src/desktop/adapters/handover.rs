//! La entrega de un fichero al escritorio lanzando otro proceso de rFirma, que la instancia única reenvía si ya hay uno.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::desktop::application::invocation::arguments_for_the_desktop;
use crate::desktop::domain::command_line::WindowIntent;
use crate::desktop::ports::DesktopHandover;

/// Lanza `rfirma <fichero>`, o `rfirma --see-signatures <fichero>`, sin esperarlo.
pub struct SpawnedDesktop;

impl DesktopHandover for SpawnedDesktop {
    fn hand_over(&self, file: &Path, intent: WindowIntent) -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let absolute = std::path::absolute(file).map_err(|error| error.to_string())?;
        Command::new(executable)
            .args(arguments_for_the_desktop(&absolute, intent))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(drop)
            .map_err(|error| error.to_string())
    }
}
