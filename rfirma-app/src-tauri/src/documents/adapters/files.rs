//! El disco de verdad tras el puerto `DocumentFiles`: `std::fs` y nada más.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::documents::domain::destination::FolderFact;
use crate::documents::ports::DocumentFiles;

/// El sistema de ficheros de esta máquina.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealFiles;

impl DocumentFiles for RealFiles {
    fn folder_fact(&self, path: &Path) -> FolderFact {
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => FolderFact::Folder,
            Ok(_) => FolderFact::NotAFolder,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => FolderFact::Missing,
            Err(error) => FolderFact::Unreadable(error.to_string()),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn readable(&self, path: &Path) -> Result<(), String> {
        std::fs::File::open(path)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|error| error.to_string())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        std::fs::write(path, bytes).map_err(|error| error.to_string())
    }

    fn modified_seconds(&self, path: &Path) -> Option<u64> {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|elapsed| elapsed.as_secs())
    }

    fn canonical(&self, path: &Path) -> Option<PathBuf> {
        std::fs::canonicalize(path).ok()
    }

    fn files_within(&self, folder: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return Vec::new();
        };
        let mut found: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect();
        found.sort();
        found
    }
}
