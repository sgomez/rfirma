//! El documento tal y como entra por el portal de documentos (ADR-0011).

use std::path::{Path, PathBuf};

const PORTAL_ROOT: &str = "/run/user";
const PORTAL_DIRECTORY: &str = "doc";

/// Documento recibido a través del portal de documentos o ruta directa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortalDocument {
    handle: PathBuf,
    name: String,
}

impl PortalDocument {
    /// Construye la representación del documento a partir de su ruta.
    pub fn opened(handle: impl Into<PathBuf>) -> Self {
        let handle = handle.into();
        let name = handle
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self { handle, name }
    }

    /// Nombre del fichero del documento.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Ruta concedida para lectura del documento (ADR-0011).
    pub fn reading_path(&self) -> &Path {
        &self.handle
    }

    /// Identificador concedido por el portal de documentos si procede.
    pub fn portal_id(&self) -> Option<&str> {
        let directory = self.handle.parent()?;
        let identifier = directory.file_name()?.to_str()?;
        let root = directory.parent()?;
        if root.file_name()? != PORTAL_DIRECTORY || !root.starts_with(PORTAL_ROOT) {
            return None;
        }
        Some(identifier)
    }

    /// Comprueba si el documento proviene del portal del sandbox.
    pub fn came_through_the_portal(&self) -> bool {
        self.portal_id().is_some()
    }
}

const SANDBOX_MARKER: &str = "/.flatpak-info";

/// Comprueba si el entorno permite ofrecer la carpeta del original como destino.
pub fn the_original_folder_can_be_offered() -> bool {
    !inside_a_sandbox(Path::new(SANDBOX_MARKER))
}

fn inside_a_sandbox(marker: &Path) -> bool {
    marker.exists()
}

#[cfg(test)]
mod tests;
