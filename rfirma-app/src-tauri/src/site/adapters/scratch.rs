//! La carpeta de paso tras el puerto `Scratch` —la del documento de la sede mientras dura el trámite, propia de cada proceso y cerrada con `flock`— y las rutas que elige la persona al guardar o cargar (ADR-0011).

use std::fs::{File, OpenOptions};
use std::io;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use crate::documents::domain::handles;
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

    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| error.to_string())
    }

    fn erase(&self, path: &Path) {
        let _ = std::fs::remove_file(path);
    }
}

/// El fichero, dentro de una carpeta de proceso, que sostiene su cerrojo (ADR-0024).
const LOCK_FILE_NAME: &str = ".lock";

/// La carpeta de paso de este proceso, con su cerrojo sostenido mientras el guardián vive.
pub struct ProcessFolder {
    path: PathBuf,
    _lock: File,
}

impl ProcessFolder {
    /// La ruta de la carpeta.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Nombra, crea y cierra con `flock` la carpeta de paso de este proceso bajo `temp`, con el
/// prefijo del rol dado.
pub fn own_folder(temp: &Path, role: &str) -> io::Result<ProcessFolder> {
    let path = temp.join(folder_name(role));
    std::fs::create_dir_all(&path)?;
    let lock = lock_exclusively(&path.join(LOCK_FILE_NAME))?;
    Ok(ProcessFolder { path, _lock: lock })
}

fn folder_name(role: &str) -> String {
    format!("rfirma-{role}-{}", handles::mint())
}

#[cfg(unix)]
fn lock_exclusively(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if locked != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

/// En Windows el cerrojo es abrir el fichero sin compartirlo: nadie más lo abre mientras vive.
#[cfg(windows)]
fn lock_exclusively(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt as _;

    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .share_mode(0)
        .open(path)
}

/// Barre, bajo `temp`, las carpetas de los prefijos dados cuyo cerrojo se consigue tomar, y las
/// borra enteras. No se barre por PID (ADR-0024): flatpak da a cada instancia su propio espacio
/// de PID.
pub fn sweep(temp: &Path, prefixes: &[&str]) {
    let Ok(entries) = std::fs::read_dir(temp) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if prefixes.iter().any(|prefix| named_with(&path, prefix))
            && lock_exclusively(&path.join(LOCK_FILE_NAME)).is_ok()
        {
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}

fn named_with(path: &Path, role: &str) -> bool {
    let prefix = format!("rfirma-{role}-");
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(&prefix))
}

#[cfg(all(test, unix))]
mod tests;
