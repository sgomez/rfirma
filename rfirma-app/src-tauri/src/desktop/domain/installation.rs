//! El resultado cerrado de instalar la versión anunciada desde la aplicación y los fallos del instalador (ADR-0015).

/// Por qué el instalador no pudo comprobar o instalar la versión anunciada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallFailure {
    /// No se pudo leer el feed o descargar el instalador.
    Network,
    /// La firma minisign del instalador no casa con la clave pública versionada.
    InvalidSignature,
    /// Este canal no instala desde la aplicación.
    NotAvailable,
}

/// Lo que pasó al pedir que se instale la versión anunciada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Installation {
    /// El instalador se lanzó sobre la instalación actual.
    Installed,
    /// No hay una versión anunciada mayor que la que corre.
    NoUpdate,
    /// No se pudo leer el feed o descargar el instalador.
    NetworkFailure,
    /// La firma del instalador no es válida y no se ejecutó.
    InvalidSignature,
    /// Este canal no instala desde la aplicación.
    NotAvailable,
}

impl From<InstallFailure> for Installation {
    fn from(failure: InstallFailure) -> Self {
        match failure {
            InstallFailure::Network => Self::NetworkFailure,
            InstallFailure::InvalidSignature => Self::InvalidSignature,
            InstallFailure::NotAvailable => Self::NotAvailable,
        }
    }
}
