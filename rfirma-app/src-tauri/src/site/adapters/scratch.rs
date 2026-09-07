//! La carpeta de paso de verdad tras el puerto `Scratch`: `std::fs` y nada más.

use std::path::Path;

use crate::site::ports::Scratch;

/// El fichero de paso en el sistema de ficheros de esta máquina.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealScratch;

impl Scratch for RealScratch {
    fn make_the_folder(&self, folder: &Path) -> Result<(), String> {
        std::fs::create_dir_all(folder).map_err(|error| error.to_string())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|error| error.to_string())
    }

    fn erase(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}
