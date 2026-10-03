//! `WindowsCredentialManager`: el PIN del Almacén de rFirma en una credencial genérica del Administrador de credenciales de Windows (ADR-0034, ADR-0035).

use std::ptr;

use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
use windows_sys::Win32::Security::Credentials::{
    CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

use crate::identity::domain::keyring::{generate_pin, KeyringError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::ports::Keyring;

const TARGET: &str = "rfirma/almacen-pin";

/// Una credencial genérica del usuario en el Administrador de credenciales.
pub struct WindowsCredentialManager {
    target: Vec<u16>,
}

impl WindowsCredentialManager {
    /// El Administrador de credenciales del usuario: siempre está.
    pub fn new() -> Result<Self, KeyringError> {
        Ok(Self::at(TARGET))
    }

    fn at(target: &str) -> Self {
        Self {
            target: target.encode_utf16().chain(std::iter::once(0)).collect(),
        }
    }
}

impl Keyring for WindowsCredentialManager {
    fn pin(&self) -> Result<ProtectedSecret, KeyringError> {
        let mut credential: *mut CREDENTIALW = ptr::null_mut();
        if unsafe { CredReadW(self.target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
            let code = std::io::Error::last_os_error().raw_os_error();
            return Err(if code == Some(ERROR_NOT_FOUND as i32) {
                KeyringError::PinMissing
            } else {
                KeyringError::NoKeyring
            });
        }
        let secret = unsafe {
            let read = &*credential;
            let blob =
                std::slice::from_raw_parts(read.CredentialBlob, read.CredentialBlobSize as usize);
            let secret = ProtectedSecret::new(blob);
            CredFree(credential.cast());
            secret
        };
        if secret.is_empty() {
            return Err(KeyringError::PinMissing);
        }
        Ok(secret)
    }

    fn create_pin(&self) -> Result<ProtectedSecret, KeyringError> {
        let pin = generate_pin();
        let mut blob = pin.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: self.target.as_ptr().cast_mut(),
            CredentialBlobSize: u32::try_from(blob.len()).map_err(|_| KeyringError::NoKeyring)?,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..CREDENTIALW::default()
        };
        let written = unsafe { CredWriteW(&credential, 0) };
        zeroize::Zeroize::zeroize(&mut blob);
        if written == 0 {
            return Err(KeyringError::NoKeyring);
        }
        Ok(pin)
    }
}

#[cfg(test)]
mod tests;
