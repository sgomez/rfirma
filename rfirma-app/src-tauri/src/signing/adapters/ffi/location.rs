//! Dónde se busca la librería nativa: `RFIRMA_LIB_DIR` y el directorio del paquete (ADR-0004, ADR-0035).

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::desktop::adapters::paths::Platform;
use crate::signing::domain::bridge::{
    Candidate, LibraryNotFound, Origin, LIBRARY_DIRECTORY_VARIABLE,
};

/// Nombre del fichero de la librería nativa en esta plataforma (ADR-0004, ADR-0035).
pub fn library_file() -> String {
    format!("{DLL_PREFIX}rfirma_crypto{DLL_SUFFIX}")
}

/// Directorios candidatos donde buscar la librería nativa en orden de prioridad.
pub fn candidates(
    environment: &dyn Fn(&str) -> Option<OsString>,
    executable_directory: &Path,
) -> Vec<Candidate> {
    let mut found = Vec::with_capacity(2);
    if let Some(value) = environment(LIBRARY_DIRECTORY_VARIABLE).filter(|value| !value.is_empty()) {
        found.push(Candidate {
            directory: PathBuf::from(value),
            origin: Origin::Override,
            file: library_file(),
        });
    }
    found.push(Candidate {
        directory: normalise(Platform::CURRENT.native_library_directory(executable_directory)),
        origin: Origin::RelativeToExecutable,
        file: library_file(),
    });
    found
}

fn normalise(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

/// Localiza el fichero de la librería nativa en los directorios candidatos.
pub fn locate(
    environment: &dyn Fn(&str) -> Option<OsString>,
    executable_directory: &Path,
) -> Result<PathBuf, LibraryNotFound> {
    let looked_at = candidates(environment, executable_directory);
    looked_at
        .iter()
        .map(Candidate::library_path)
        .find(|path| path.is_file())
        .ok_or(LibraryNotFound { looked_at })
}
