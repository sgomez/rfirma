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
mod tests {
    use super::*;

    fn a_reference(label: &str, cka_id: u8) -> CertificateRef {
        CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            label,
            vec![cka_id],
        )
    }

    #[test]
    fn a_listed_certificate_comes_back_by_its_handle() {
        let listed = ListedCertificates::new();

        let handles = listed.replace([a_reference("FIRMA", 0x01)]);

        assert_eq!(handles.len(), 1);
        assert_eq!(listed.get(&handles[0]), Some(a_reference("FIRMA", 0x01)));
    }

    #[test]
    fn two_certificates_with_the_same_label_get_different_handles() {
        let listed = ListedCertificates::new();

        let handles = listed.replace([
            a_reference("FNMT-GEMELO-99999999R", 0x04),
            a_reference("FNMT-GEMELO-99999999R", 0x05),
        ]);

        assert_ne!(handles[0], handles[1]);
        assert_eq!(
            listed.get(&handles[1]),
            Some(a_reference("FNMT-GEMELO-99999999R", 0x05))
        );
    }

    #[test]
    fn a_handle_nobody_minted_is_simply_not_there() {
        let listed = ListedCertificates::new();

        assert_eq!(listed.get("00000000000000000000000000000000"), None);
        assert!(listed.is_empty());
    }

    #[test]
    fn listing_again_replaces_what_the_window_can_point_at() {
        let listed = ListedCertificates::new();
        let before = listed.replace([a_reference("FIRMA", 0x01)]);

        let after = listed.replace([a_reference("OTRO", 0x02)]);

        assert_eq!(listed.len(), 1);
        assert_eq!(listed.get(&before[0]), None);
        assert_eq!(listed.get(&after[0]), Some(a_reference("OTRO", 0x02)));
    }

    #[test]
    fn the_handle_carries_nothing_of_the_certificate_it_stands_for() {
        let listed = ListedCertificates::new();

        let handles = listed.replace([a_reference("FNMT-ACTIVO-99999999R", 0x01)]);
        let handle = &handles[0];

        assert_eq!(handle.len(), 32);
        assert!(handle
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
        for leak in ["/", "usr", "softhsm", "rfirma-test", "FNMT", "99999999R"] {
            assert!(
                !handle.contains(leak),
                "el asa «{handle}» lleva «{leak}» dentro"
            );
        }
    }
}
