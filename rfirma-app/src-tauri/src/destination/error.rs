//! Clasificación de situaciones de fallo del destino (ADR-0009, ADR-0011).

use std::fmt;
use std::path::Path;

/// Situaciones de fallo del destino traducibles por el catálogo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// La carpeta de destino no existe en el anfitrión.
    FolderMissing,
    /// La ruta no es una carpeta.
    NotAFolder,
    /// No se ha podido consultar la ruta de destino.
    FolderUnreadable,
    /// Todos los nombres derivados están ocupados.
    NoFreeName,
}

/// Fallo del destino clasificado con situación y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestinationError {
    situation: Situation,
    detail: String,
}

impl DestinationError {
    /// Construye un fallo con su situación y detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// Construye un fallo asociando una ruta concreta.
    pub fn about(situation: Situation, path: &Path) -> Self {
        Self::new(situation, path.display().to_string())
    }

    /// Construye un fallo asociando una ruta y un error de E/S del sistema.
    pub fn caused_by(situation: Situation, path: &Path, error: &std::io::Error) -> Self {
        Self::new(situation, format!("{}: {error}", path.display()))
    }

    /// Situación clasificada para la interfaz.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// Detalle técnico del fallo.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for DestinationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for DestinationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::path::PathBuf;

    #[test]
    fn a_missing_folder_names_the_path_it_could_not_find() {
        let error =
            DestinationError::about(Situation::FolderMissing, &PathBuf::from("/home/quien/Docs"));

        assert_eq!(error.situation(), Situation::FolderMissing);
        assert_eq!(error.detail(), "/home/quien/Docs");
        assert!(error.to_string().contains("FolderMissing"));
    }

    #[test]
    fn an_unreadable_folder_drags_the_system_error_along() {
        let error = DestinationError::caused_by(
            Situation::FolderUnreadable,
            &PathBuf::from("/mnt/red/Docs"),
            &std::io::Error::new(ErrorKind::PermissionDenied, "denegado"),
        );

        assert!(error.detail().contains("/mnt/red/Docs"));
        assert!(error.detail().contains("denegado"));
    }
}
