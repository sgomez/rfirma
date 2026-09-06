//! Persistencia en disco de la CA local y su solape (ADR-0005).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::authority::LocalCa;
use super::error::{Situation, TlsError};
use crate::paths::{create_owner_only_file, restrict_to_owner, Paths};

/// Par de ficheros en disco de una CA local: certificado y clave privada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaFiles {
    certificate_path: PathBuf,
    key_path: PathBuf,
}

impl CaFiles {
    /// Construye una ranura de ficheros a partir de las rutas de certificado y clave.
    pub fn new(certificate_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            certificate_path: certificate_path.into(),
            key_path: key_path.into(),
        }
    }

    /// Ruta del fichero de certificado.
    pub fn certificate_path(&self) -> &Path {
        &self.certificate_path
    }

    fn read(&self) -> Result<Option<LocalCa>, TlsError> {
        if !self.certificate_path.exists() || !self.key_path.exists() {
            return Ok(None);
        }
        let certificate = read_file(&self.certificate_path)?;
        let key = read_file(&self.key_path)?;
        LocalCa::from_pem(&certificate, &key).map(Some)
    }

    fn write(&self, ca: &LocalCa) -> Result<(), TlsError> {
        let unwritable = |path: &Path, error: std::io::Error| {
            TlsError::new(
                Situation::MaterialUnwritable,
                format!("{}: {error}", path.display()),
            )
        };

        if let Some(parent) = self.key_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(parent, error))?;
            let _ = restrict_to_owner(parent);
        }

        let mut file = create_owner_only_file(&self.key_path)
            .map_err(|error| unwritable(&self.key_path, error))?;
        file.write_all(&ca.private_key_pem()?)
            .map_err(|error| unwritable(&self.key_path, error))?;

        std::fs::write(&self.certificate_path, ca.certificate_pem()?)
            .map_err(|error| unwritable(&self.certificate_path, error))
    }

    fn empty(&self) -> Result<(), TlsError> {
        let unwritable = |path: &Path, error: std::io::Error| {
            TlsError::new(
                Situation::MaterialUnwritable,
                format!("{}: {error}", path.display()),
            )
        };
        for path in [&self.certificate_path, &self.key_path] {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(unwritable(path, error)),
            }
        }
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, TlsError> {
    std::fs::read(path).map_err(|error| {
        TlsError::new(
            Situation::MaterialDamaged,
            format!("{}: {error}", path.display()),
        )
    })
}

/// Almacén de la CA local vigente y la siguiente para el periodo de solape (ADR-0005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalCaStore {
    serving: CaFiles,
    next: CaFiles,
}

impl LocalCaStore {
    /// Crea el almacén con las ranuras de la CA vigente y siguiente.
    pub fn new(serving: CaFiles, next: CaFiles) -> Self {
        Self { serving, next }
    }

    /// Construye el almacén a partir de las rutas de la aplicación.
    pub fn of(paths: &Paths) -> Self {
        Self::new(
            CaFiles::new(paths.local_ca_certificate_path(), paths.local_ca_key_path()),
            CaFiles::new(
                paths.next_local_ca_certificate_path(),
                paths.next_local_ca_key_path(),
            ),
        )
    }

    /// Ruta del certificado de la CA local vigente.
    pub fn certificate_path(&self) -> &Path {
        self.serving.certificate_path()
    }

    /// Lee la CA local vigente si existe.
    pub fn read(&self) -> Result<Option<LocalCa>, TlsError> {
        self.serving.read()
    }

    /// Guarda la CA local vigente sustituyendo la anterior.
    pub fn write(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.serving.write(ca)
    }

    /// Lee la CA local siguiente si existe.
    pub fn read_next(&self) -> Result<Option<LocalCa>, TlsError> {
        self.next.read()
    }

    /// Guarda la CA local siguiente sin modificar la vigente.
    pub fn write_next(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.next.write(ca)
    }

    /// Promueve la CA local siguiente a vigente y vacía su ranura.
    pub fn promote_next(&self) -> Result<Option<LocalCa>, TlsError> {
        let Some(next) = self.next.read()? else {
            return Ok(None);
        };
        self.serving.write(&next)?;
        self.next.empty()?;
        Ok(Some(next))
    }

    /// Elimina la CA local siguiente.
    pub fn forget_next(&self) -> Result<(), TlsError> {
        self.next.empty()
    }
}

#[cfg(test)]
mod tests;
