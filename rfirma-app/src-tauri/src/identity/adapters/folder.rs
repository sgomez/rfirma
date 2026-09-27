//! La carpeta del Almacén de rFirma y el directorio desechable de prueba, tras su puerto: `std::fs` y los permisos del dueño.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::identity::ports::InstalledFolder;

/// Los almacenes instalados en el sistema de ficheros de esta máquina.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealInstalledFolder;

impl InstalledFolder for RealInstalledFolder {
    fn make(&self, directory: &Path) -> Result<(), String> {
        std::fs::create_dir_all(directory).map_err(|error| error.to_string())
    }

    fn restrict_to_owner(&self, path: &Path) {
        let _ = crate::desktop::adapters::paths::restrict_to_owner(path);
    }

    fn remove(&self, directory: &Path) -> Result<(), String> {
        std::fs::remove_dir_all(directory).map_err(|error| error.to_string())
    }

    fn remove_file(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn staging_directory(&self) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("rfirma-p12-check-{}-{unique}", std::process::id()))
    }
}
