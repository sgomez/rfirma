//! El documento en curso: por dónde entró, por dónde se lee y si de él queda rastro (ADR-0011).

use std::path::{Path, PathBuf};

const PORTAL_ROOT: &str = "/run/user";
const PORTAL_DIRECTORY: &str = "doc";

/// Por dónde entró el documento (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Por el portal de documentos del sandbox, con el identificador que este le dio.
    Portal(String),
    /// Por una ruta del anfitrión: el diálogo fuera del sandbox, lo soltado o la línea de órdenes.
    Host,
}

/// Si del documento se guarda rastro en la bandeja.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Remembrance {
    /// Documento local que genera historial y recuerda estado.
    Remembered,
    /// Documento efímero o de sede del que no se guarda rastro.
    Unrecorded,
}

/// El documento en curso, tal como entró y con lo que se decide de él.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    origin: Origin,
    reading_path: PathBuf,
    name: String,
    remembrance: Remembrance,
}

impl Document {
    /// El documento que entra por la puerta que recuerda.
    pub fn opened(reading_path: impl Into<PathBuf>) -> Self {
        Self::entered(reading_path.into(), Remembrance::Remembered)
    }

    /// El documento de paso, del que no queda rastro.
    pub fn passing_through(reading_path: impl Into<PathBuf>) -> Self {
        Self::entered(reading_path.into(), Remembrance::Unrecorded)
    }

    fn entered(reading_path: PathBuf, remembrance: Remembrance) -> Self {
        let name = reading_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let origin = portal_id_in(&reading_path).map_or(Origin::Host, Origin::Portal);
        Self {
            origin,
            reading_path,
            name,
            remembrance,
        }
    }

    /// Nombre del fichero del documento.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Por dónde se leen sus bytes. **No cruza a la ventana** (ADR-0011).
    pub fn reading_path(&self) -> &Path {
        &self.reading_path
    }

    /// Por dónde entró.
    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// Identificador concedido por el portal de documentos si entró por él.
    pub fn portal_id(&self) -> Option<&str> {
        match &self.origin {
            Origin::Portal(id) => Some(id),
            Origin::Host => None,
        }
    }

    /// Si entró por el portal del sandbox.
    pub fn came_through_the_portal(&self) -> bool {
        matches!(self.origin, Origin::Portal(_))
    }

    /// Si de él se guarda rastro en la bandeja.
    pub fn is_remembered(&self) -> bool {
        self.remembrance == Remembrance::Remembered
    }
}

fn portal_id_in(reading_path: &Path) -> Option<String> {
    let directory = reading_path.parent()?;
    let identifier = directory.file_name()?.to_str()?;
    let root = directory.parent()?;
    if root.file_name()? != PORTAL_DIRECTORY || !root.starts_with(PORTAL_ROOT) {
        return None;
    }
    Some(identifier.to_owned())
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
