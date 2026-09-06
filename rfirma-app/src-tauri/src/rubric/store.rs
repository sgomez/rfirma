//! Almacén persistente de la rúbrica normalizada (ADR-0010, ADR-0012).

use std::fs::{self, File};
use std::io::Read as _;
use std::path::{Path, PathBuf};

use super::error::{RubricError, Situation};
use super::normalize::{normalize, NormalizedRubric, MAX_INPUT_BYTES};

/// Almacén persistente de la rúbrica normalizada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RubricStore {
    path: PathBuf,
}

impl RubricStore {
    /// Construye un almacén en la ruta indicada.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Ruta del fichero de rúbrica en el almacén.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lee, normaliza y persiste la rúbrica indicada (ADR-0010, ADR-0011).
    pub fn adopt(&self, source: &Path) -> Result<NormalizedRubric, RubricError> {
        let named = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let unreadable = |error: std::io::Error| {
            RubricError::new(Situation::SourceUnreadable, format!("{named}: {error}"))
        };

        let mut bytes = Vec::new();
        File::open(source)
            .map_err(&unreadable)?
            .take(MAX_INPUT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(unreadable)?;
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(RubricError::new(
                Situation::ImageTooLarge,
                format!("{named} pasa del tope de {MAX_INPUT_BYTES} bytes"),
            ));
        }

        let normalized = normalize(&bytes)?;
        self.save(&normalized)?;
        Ok(normalized)
    }

    /// Persiste la rúbrica normalizada en el almacén de forma atómica (ADR-0010).
    pub fn save(&self, rubric: &NormalizedRubric) -> Result<(), RubricError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| self.unwritable(&error))?;
        }
        let temporary = self.path.with_extension("jpg.tmp");
        fs::write(&temporary, rubric.bytes()).map_err(|error| self.unwritable(&error))?;
        fs::rename(&temporary, &self.path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            self.unwritable(&error)
        })
    }

    /// Lee la rúbrica guardada en el almacén si existe.
    pub fn stored(&self) -> Result<Option<Vec<u8>>, RubricError> {
        match fs::read(&self.path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(RubricError::new(
                Situation::StoreUnreadable,
                format!("{}: {error}", self.path.display()),
            )),
        }
    }

    fn unwritable(&self, error: &std::io::Error) -> RubricError {
        RubricError::new(
            Situation::StoreUnwritable,
            format!("{}: {error}", self.path.display()),
        )
    }
}

#[cfg(test)]
mod tests;
