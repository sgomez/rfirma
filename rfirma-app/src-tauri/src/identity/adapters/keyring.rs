//! El llavero del escritorio para el PIN del Almacén de rFirma, con `oo7` (ADR-0034).

use std::collections::HashMap;

use crate::identity::domain::keyring::{generate_pin, KeyringError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::ports::Keyring;

const ITEM_LABEL: &str = "Almacén de rFirma";

fn item_attributes() -> HashMap<&'static str, &'static str> {
    HashMap::from([("purpose", "rfirma-almacen-pin")])
}

/// El llavero del escritorio, alcanzado por el portal de secretos o Secret Service con `oo7`.
pub struct RealKeyring {
    backend: oo7::Keyring,
}

impl RealKeyring {
    /// Detecta el portal de secretos dentro del flatpak, o Secret Service por D-Bus fuera de él.
    pub fn new() -> Result<Self, KeyringError> {
        let backend = tauri::async_runtime::block_on(oo7::Keyring::new())
            .map_err(|_| KeyringError::NoKeyring)?;
        Ok(Self { backend })
    }

    #[cfg(test)]
    fn from_backend(backend: oo7::Keyring) -> Self {
        Self { backend }
    }
}

impl Keyring for RealKeyring {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        tauri::async_runtime::block_on(async {
            let items = self
                .backend
                .search_items(&item_attributes())
                .await
                .map_err(|_| KeyringError::NoKeyring)?;
            let item = items.first().ok_or(KeyringError::PinMissing)?;
            let secret = item.secret().await.map_err(|_| KeyringError::NoKeyring)?;
            Ok(ProtectedSecret::new(secret.as_ref()))
        })
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        let pin = generate_pin();
        tauri::async_runtime::block_on(self.backend.create_item(
            ITEM_LABEL,
            &item_attributes(),
            pin.as_bytes(),
            true,
        ))
        .map_err(|_| KeyringError::NoKeyring)?;
        Ok(pin)
    }
}

#[cfg(test)]
mod tests;
