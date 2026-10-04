//! El sistema operativo en el que corre la aplicación, sin saber cómo se detecta.

/// Plataformas soportadas para la resolución de rutas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Entorno Linux basado en estándares XDG.
    Linux,
    /// Entorno Windows basado en perfiles de usuario.
    Windows,
    /// Entorno macOS basado en Application Support.
    MacOs,
}
