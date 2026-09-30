//! Puertos del contexto de escritorio: el registro de manejadores, la memoria de la versión publicada y su instalador.

use crate::desktop::domain::error::DesktopError;
use crate::desktop::domain::handlers::UrlHandler;
use crate::desktop::domain::installation::InstallFailure;
use crate::desktop::domain::version_check::VersionCheck;
use crate::memory_error::MemoryError;

/// Quién atiende un esquema según el escritorio, y cómo se elige (ADR-0015).
pub trait HandlerRegistry {
    /// Los manejadores registrados para el esquema, o nada si el sandbox no deja preguntar.
    fn registered_for(&self, scheme: &str) -> Option<Vec<UrlHandler>>;

    /// El manejador que la persona tiene elegido para el esquema, si eligió alguno.
    fn current_default_for(&self, scheme: &str) -> Option<String>;

    /// Deja ese manejador como elegido para el esquema.
    fn choose_for(&self, scheme: &str, handler: &str) -> Result<(), DesktopError>;

    /// Quita la clave del esquema, sin escribir ningún otro manejador en su lugar.
    fn remove_for(&self, scheme: &str) -> Result<(), DesktopError>;
}

/// La última comprobación de versión, recordada entre sesiones y exenta de los interruptores (ADR-0010).
pub trait VersionMemory {
    /// La última comprobación guardada, si la hay.
    fn last_version_check(&self) -> Option<VersionCheck>;

    /// Guarda la comprobación recién hecha.
    fn remember_version_check(&self, check: VersionCheck) -> Result<(), MemoryError>;
}

/// Descarga, verifica e instala la versión anunciada; fuera de Windows, no está disponible (ADR-0035).
pub trait UpdateInstaller {
    /// La versión que anuncia el feed de instalación, o nada si no hay una mayor.
    fn announced(&self) -> Result<Option<String>, InstallFailure>;

    /// Descarga la versión anunciada, verifica su firma, cierra la aplicación y ejecuta el instalador.
    fn install(&self) -> Result<(), InstallFailure>;
}
