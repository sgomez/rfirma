//! El llavero de Windows, que aún no existe: sin Administrador de credenciales no hay llavero (ADR-0035).

use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::ports::Keyring;

/// El adaptador de `Keyring` pendiente sobre el Administrador de credenciales de Windows.
pub struct PendingWindowsCredentialManager;

impl PendingWindowsCredentialManager {
    /// Siempre falla con `NoKeyring`, igual que un Linux sin Secret Service.
    pub fn new() -> Result<Self, KeyringError> {
        Err(KeyringError::NoKeyring)
    }
}

impl Keyring for PendingWindowsCredentialManager {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::NoKeyring)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::NoKeyring)
    }
}
