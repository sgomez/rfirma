//! Registro en memoria de certificados listados en la sesión activa (ADR-0011).

use std::collections::HashMap;
use std::sync::Mutex;

use super::handles::mint;
use crate::pkcs11::CertificateRef;

/// Certificados del último listado indexados por su identificador opaco.
#[derive(Debug, Default)]
pub struct ListedCertificates {
    certificates: Mutex<HashMap<String, CertificateRef>>,
}

impl ListedCertificates {
    /// Construye una colección vacía de certificados listados.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra un listado de certificados y retorna sus identificadores opacos (ADR-0011).
    pub fn replace(&self, references: impl IntoIterator<Item = CertificateRef>) -> Vec<String> {
        let minted: Vec<(String, CertificateRef)> = references
            .into_iter()
            .map(|reference| (mint(), reference))
            .collect();
        let handles = minted.iter().map(|(handle, _)| handle.clone()).collect();
        *lock(&self.certificates) = minted.into_iter().collect();
        handles
    }

    /// Obtiene la referencia asociada a un identificador opaco si existe.
    pub fn get(&self, handle: &str) -> Option<CertificateRef> {
        lock(&self.certificates).get(handle).cloned()
    }

    /// Cantidad de certificados registrados en el listado activo.
    pub fn len(&self) -> usize {
        lock(&self.certificates).len()
    }

    /// Indica si el registro está vacío.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
