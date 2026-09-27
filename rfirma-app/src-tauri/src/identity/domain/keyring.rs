//! El PIN del Almacén de rFirma: aleatorio y largo, y por qué el llavero no lo entregó (ADR-0034).

use std::fmt;

use super::protected_secret::ProtectedSecret;

/// Bytes de aleatoriedad del PIN generado (256 bits), antes de codificarlo en hexadecimal.
const PIN_RANDOM_BYTES: usize = 32;

/// Genera un PIN nuevo para el Almacén de rFirma: aleatorio, largo, listo para guardarlo en el llavero.
pub fn generate_pin() -> ProtectedSecret {
    let mut bytes = [0u8; PIN_RANDOM_BYTES];
    getrandom::fill(&mut bytes).expect("el generador de aleatoriedad del sistema no falla");
    ProtectedSecret::from_str(&as_hex(&bytes))
}

fn as_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Por qué el llavero del escritorio no entregó el PIN del Almacén de rFirma.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyringError {
    /// No hay portal de secretos ni Secret Service disponible.
    NoKeyring,
    /// El llavero está disponible pero no tiene el PIN, o lo perdió.
    PinMissing,
}

impl fmt::Display for KeyringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoKeyring => write!(f, "no hay llavero del escritorio disponible"),
            Self::PinMissing => write!(f, "el llavero no tiene el PIN del almacén"),
        }
    }
}

impl std::error::Error for KeyringError {}

#[cfg(test)]
mod tests;
