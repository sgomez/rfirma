//! Memoria protegida para secretos (PIN, contraseñas) con bloqueo en RAM y borrado seguro (ADR-0001, ADR-0014).

use std::fmt;
use std::ops::Deref;
use std::str::Utf8Error;
use zeroize::Zeroize;

/// Búfer de memoria protegida para secretos sensibles como el PIN.
///
/// Bloquea el rango de memoria física asignada mediante `mlock(2)` y previene
/// su inclusión en volcados de memoria con `MADV_DONTDUMP`.
/// Al destruirse (`Drop`), limpia la memoria de forma segura mediante `zeroize`
/// antes de liberar el bloqueo con `munlock(2)`.
pub struct ProtectedSecret {
    bytes: Vec<u8>,
    locked: bool,
}

impl ProtectedSecret {
    /// Crea un nuevo búfer protegido copiando los bytes suministrados.
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        let mut bytes = secret.as_ref().to_vec();
        let len = bytes.len();
        let mut locked = false;

        if len > 0 {
            unsafe {
                let ptr = bytes.as_mut_ptr() as *mut libc::c_void;
                let cap = bytes.capacity();
                if libc::mlock(ptr, cap) == 0 {
                    locked = true;
                    libc::madvise(ptr, cap, libc::MADV_DONTDUMP);
                }
            }
        }

        Self { bytes, locked }
    }

    /// Crea un nuevo secreto protegido a partir de una cadena UTF-8.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(secret: &str) -> Self {
        Self::new(secret.as_bytes())
    }

    /// Comprueba si la memoria física fue bloqueada con éxito mediante `mlock`.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Devuelve el contenido como una porción de bytes (`&[u8]`).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Interpreta el secreto como una cadena UTF-8, o error si contiene bytes no válidos.
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(&self.bytes)
    }

    /// Expone el secreto como `&str`.
    pub fn expose_secret(&self) -> Result<&str, Utf8Error> {
        self.as_str()
    }

    /// Longitud en bytes del secreto protegido.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Indica si el secreto está vacío.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Limpia de forma segura el contenido del búfer con ceros.
    pub fn wipe(&mut self) {
        self.bytes.zeroize();
    }
}

impl Deref for ProtectedSecret {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl From<&str> for ProtectedSecret {
    fn from(secret: &str) -> Self {
        Self::from_str(secret)
    }
}

impl Drop for ProtectedSecret {
    fn drop(&mut self) {
        self.bytes.zeroize();
        if self.locked && !self.bytes.is_empty() {
            unsafe {
                let ptr = self.bytes.as_mut_ptr() as *mut libc::c_void;
                let cap = self.bytes.capacity();
                libc::munlock(ptr, cap);
            }
        }
    }
}

impl fmt::Debug for ProtectedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ProtectedSecret([REDACTED])")
    }
}

impl PartialEq for ProtectedSecret {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for ProtectedSecret {}

#[cfg(test)]
mod tests;
