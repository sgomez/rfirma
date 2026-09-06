//! Puertos del contexto de identidad.

use libloading::Library;

use crate::identity::domain::error::NssUnavailable;

/// Puerto para interactuar con la biblioteca NSS y el turno global del token.
pub trait NssHost {
    /// Biblioteca `libnss3.so` del sistema cargada en memoria.
    fn library(&self) -> Result<&'static Library, NssUnavailable>;

    /// Ejecuta una operación bajo el turno global del token.
    fn with_token_turn<T>(&self, work: impl FnOnce() -> T) -> T;
}
