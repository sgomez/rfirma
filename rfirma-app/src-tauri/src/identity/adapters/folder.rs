//! La carpeta de los `.p12` instalados tras su puerto: `std::fs` y los permisos del dueño.

use std::path::Path;

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
}
