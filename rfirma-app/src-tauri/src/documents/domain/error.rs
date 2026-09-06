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

/// Por qué un documento no se ha podido abrir, leer o entregar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentError {
    /// El documento no está abierto o no se ha podido leer.
    Unreadable(String),
    /// La carpeta de destino no vale (ADR-0011).
    Destination(DestinationError),
    /// La carpeta de destino no ha dejado escribir el fichero firmado.
    FolderUnwritable(String),
}

impl DocumentError {
    /// El documento cuyo identificador ya no está abierto en esta sesión.
    pub fn no_longer_open() -> Self {
        Self::Unreadable("el documento ya no esta abierto en esta sesion".to_owned())
    }
}

impl From<DestinationError> for DocumentError {
    fn from(error: DestinationError) -> Self {
        Self::Destination(error)
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(detail) | Self::FolderUnwritable(detail) => f.write_str(detail),
            Self::Destination(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DocumentError {}

#[cfg(test)]
mod tests;
