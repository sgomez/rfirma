//! Sello de sesión opaco entre prefirma y postfirma (ADR-0016).

use std::fmt;

/// Bloque opaco que devuelve la prefirma y que la postfirma exige idéntico.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionSeal(String);

impl SessionSeal {
    /// Recoge el sello tal y como lo devolvió la prefirma.
    pub fn from_bridge(payload: impl Into<String>) -> Self {
        Self(payload.into())
    }

    /// Devuelve el sello tal cual, para dárselo a la postfirma.
    pub fn as_bridge_payload(&self) -> &str {
        &self.0
    }

    /// Comprueba, **antes de firmar**, que el sello que va a recibir la
    /// postfirma es exactamente el que produjo la prefirma.
    pub fn verify_unchanged(&self, returned: &Self) -> Result<(), SealMismatch> {
        if self == returned {
            Ok(())
        } else {
            Err(SealMismatch)
        }
    }
}

impl fmt::Debug for SessionSeal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SessionSeal({} bytes)", self.0.len())
    }
}

/// El sello que llega a la postfirma no es el que produjo la prefirma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealMismatch;

impl fmt::Display for SealMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("el sello de sesión de la postfirma no es el de la prefirma")
    }
}

impl std::error::Error for SealMismatch {}

#[cfg(test)]
mod tests;
