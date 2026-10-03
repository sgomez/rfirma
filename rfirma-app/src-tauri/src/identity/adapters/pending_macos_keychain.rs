//! `PendingMacosKeychain`, el llavero de macOS que aún no existe: sin Keychain Services falla como un Linux sin Secret Service (ADR-0035, ADR-0040).

use crate::identity::domain::keyring::KeyringError;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::ports::Keyring;

/// El adaptador de `Keyring` pendiente sobre el llavero de macOS.
pub struct PendingMacosKeychain;

impl PendingMacosKeychain {
    /// Siempre falla con `NoKeyring`, igual que un Linux sin Secret Service.
    pub fn new() -> Result<Self, KeyringError> {
        Err(KeyringError::NoKeyring)
    }
}

impl Keyring for PendingMacosKeychain {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::NoKeyring)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        Err(KeyringError::NoKeyring)
    }
}
