//! **Los certificados listados en esta sesión**: del asa opaca a la referencia
//! con la que se vuelve a encontrar el certificado.
//!
//! Es el gemelo de [`super::opened`], y existe por la misma razón: lo que la
//! ventana necesita es **poder señalar una fila**, no saber de dónde salió. La
//! [`CertificateRef`] entera no puede cruzar —lleva la ruta del módulo PKCS#11
//! y el `configdir` del perfil de Firefox, que son rutas del anfitrión
//! (ADR-0011)—, así que cruza un asa acuñada aquí y la referencia se queda en
//! el backend.
//!
//! # Por qué un asa y no la etiqueta
//!
//! Porque **las etiquetas se repiten de verdad**: dos claves con el mismo
//! `CKA_LABEL` en un perfil de Firefox, dos `FNMT-GEMELO-99999999R` en el token
//! de pruebas. Buscando por etiqueta se coge siempre el primero, así que con
//! dos iguales el segundo era **inelegible**: se enseñaba en la lista y firmaba
//! el otro.
//!
//! # Por qué se reemplaza y no se acumula
//!
//! Cada listado es la verdad de ese instante y sustituye al anterior: la
//! ventana solo puede señalar filas del listado que tiene delante, y guardar
//! los de listados viejos dejaría vivas asas que apuntan a certificados que ya
//! no están.

use std::collections::HashMap;
use std::sync::Mutex;

use super::handles::mint;
use crate::pkcs11::CertificateRef;

/// Los certificados del último listado, por su asa.
#[derive(Debug, Default)]
pub struct ListedCertificates {
    certificates: Mutex<HashMap<String, CertificateRef>>,
}

impl ListedCertificates {
    /// Vacío, que es como arranca la aplicación.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apunta un listado recién hecho y devuelve **sus asas**, en el mismo
    /// orden en que llegaron las referencias.
    ///
    /// Sustituye al listado anterior entero. Dos certificados con la misma
    /// etiqueta reciben asas distintas, que es justo lo que los hace elegibles
    /// por separado.
    pub fn replace(&self, references: impl IntoIterator<Item = CertificateRef>) -> Vec<String> {
        let minted: Vec<(String, CertificateRef)> = references
            .into_iter()
            .map(|reference| (mint(), reference))
            .collect();
        let handles = minted.iter().map(|(handle, _)| handle.clone()).collect();
        *lock(&self.certificates) = minted.into_iter().collect();
        handles
    }

    /// La referencia que se apuntó con esa asa, si sigue en el último listado.
    pub fn get(&self, handle: &str) -> Option<CertificateRef> {
        lock(&self.certificates).get(handle).cloned()
    }

    /// Cuántas hay apuntadas.
    pub fn len(&self) -> usize {
        lock(&self.certificates).len()
    }

    /// Si no hay ninguna.
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

    /// **Grada A**: el registro es una tabla en memoria y un acuñado. Contra
    /// etiquetas repetidas **de verdad** lo prueban `tests/pkcs11_token.rs` y
    /// `tests/nss_store.rs`, que son grada B.
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

    /// El caso que hace falta el asa: dos certificados con la **misma
    /// etiqueta** son dos filas distintas y cada una lleva a la suya.
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

    /// Volver a buscar sustituye el listado: las asas del anterior dejan de
    /// valer en vez de quedarse apuntando a lo que ya no está.
    #[test]
    fn listing_again_replaces_what_the_window_can_point_at() {
        let listed = ListedCertificates::new();
        let before = listed.replace([a_reference("FIRMA", 0x01)]);

        let after = listed.replace([a_reference("OTRO", 0x02)]);

        assert_eq!(listed.len(), 1);
        assert_eq!(listed.get(&before[0]), None);
        assert_eq!(listed.get(&after[0]), Some(a_reference("OTRO", 0x02)));
    }

    /// La invariante del ADR-0011 en el sitio donde se acuña: del asa no sale
    /// ni un trozo de la ruta del módulo, ni de la etiqueta, ni del token.
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
