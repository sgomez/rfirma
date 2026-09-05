//! Dónde vive la CA local entre dos arranques (ID-221, ID-223, ADR-0005).
//!
//! Son **dos ficheros** y no un PKCS#12: un `.p12` habría que cifrarlo con una
//! contraseña, y la contraseña acabaría en el código —que es exactamente el
//! `KS_PASSWORD = "654321"` de AutoFirma—. Así que el certificado va en PEM
//! corriente y la clave privada en un PEM sin cifrar dentro de un fichero
//! `0600`, el mismo trato que `~/.ssh/id_*`. Quien ya ejecuta código como la
//! persona está **declarado fuera del modelo de amenaza** por el ADR-0005: ese
//! atacante escribe él en el `nssdb` y planta una raíz sin restricciones ni
//! caducidad, que es estrictamente más poderoso que robarnos esta.
//!
//! El almacén **no decide nada**: no renueva, no reinstala y no elige entre dos
//! CA locales vivas. Solo lee lo que hay y escribe lo que le den. El solape y
//! la renovación son del caso de uso que registra en los almacenes NSS.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::authority::LocalCa;
use super::error::{Situation, TlsError};
use crate::paths::{create_owner_only_file, restrict_to_owner, Paths};

/// Los dos ficheros de la CA local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalCaStore {
    certificate_path: PathBuf,
    key_path: PathBuf,
}

impl LocalCaStore {
    /// El almacén que vive en esas dos rutas.
    pub fn new(certificate_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            certificate_path: certificate_path.into(),
            key_path: key_path.into(),
        }
    }

    /// El almacén en el directorio de datos de la aplicación.
    pub fn of(paths: &Paths) -> Self {
        Self::new(paths.local_ca_certificate_path(), paths.local_ca_key_path())
    }

    /// La ruta del certificado, que es lo que se registra en los almacenes NSS.
    pub fn certificate_path(&self) -> &Path {
        &self.certificate_path
    }

    /// La CA local que había, o `None` si todavía no hay ninguna.
    ///
    /// La ausencia **no es un fallo**: un `$HOME` sin CA local es el primer
    /// arranque. Lo que sí lo es —y se dice— es que los ficheros estén y no se
    /// entiendan.
    pub fn read(&self) -> Result<Option<LocalCa>, TlsError> {
        if !self.certificate_path.exists() || !self.key_path.exists() {
            return Ok(None);
        }
        let certificate = self.read_file(&self.certificate_path)?;
        let key = self.read_file(&self.key_path)?;
        LocalCa::from_pem(&certificate, &key).map(Some)
    }

    /// Guarda la CA local, sustituyendo la que hubiera.
    ///
    /// La clave se crea **ya** con el modo `0600` y no con un `chmod` posterior
    /// (ADR-0005): entre el `create` y el `chmod` la clave privada estaría
    /// legible para cualquier cuenta de la máquina.
    pub fn write(&self, ca: &LocalCa) -> Result<(), TlsError> {
        let unwritable = |path: &Path, error: std::io::Error| {
            TlsError::new(
                Situation::MaterialUnwritable,
                format!("{}: {error}", path.display()),
            )
        };

        if let Some(parent) = self.key_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| unwritable(parent, error))?;
            let _ = restrict_to_owner(parent);
        }

        let mut file = create_owner_only_file(&self.key_path)
            .map_err(|error| unwritable(&self.key_path, error))?;
        file.write_all(&ca.private_key_pem()?)
            .map_err(|error| unwritable(&self.key_path, error))?;

        std::fs::write(&self.certificate_path, ca.certificate_pem()?)
            .map_err(|error| unwritable(&self.certificate_path, error))
    }

    fn read_file(&self, path: &Path) -> Result<Vec<u8>, TlsError> {
        std::fs::read(path).map_err(|error| {
            TlsError::new(
                Situation::MaterialDamaged,
                format!("{}: {error}", path.display()),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_in(directory: &Path) -> LocalCaStore {
        LocalCaStore::of(&Paths::under(directory))
    }

    #[test]
    fn the_first_boot_finds_no_local_ca_and_that_is_not_a_failure() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");

        let found = store_in(directory.path())
            .read()
            .expect("no haber nada no es un fallo");

        assert!(found.is_none());
    }

    #[test]
    fn the_local_ca_survives_a_restart() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = store_in(directory.path());
        let ca = LocalCa::generate().expect("deberia generarse");

        store.write(&ca).expect("deberia guardarse");
        let restored = store
            .read()
            .expect("deberia leerse")
            .expect("la CA local se conserva entre arranques (ID-221)");

        assert_eq!(
            restored.certificate().to_pem().unwrap(),
            ca.certificate().to_pem().unwrap()
        );
    }

    #[test]
    fn a_local_ca_that_no_longer_parses_is_said_out_loud() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = store_in(directory.path());
        store
            .write(&LocalCa::generate().expect("deberia generarse"))
            .expect("deberia guardarse");
        std::fs::write(store.certificate_path(), b"esto no es un PEM").expect("deberia escribirse");

        let error = store.read().expect_err("un PEM roto no es 'no hay nada'");

        assert_eq!(error.situation(), Situation::MaterialDamaged);
    }

    // El modo `0600` del fichero de la clave **no se comprueba aquí**: leerlo
    // pide `std::os::unix`, y el ID-35 deja ese conocimiento en un solo
    // fichero. La prueba vive junto a la puerta que lo pone, en `paths.rs`
    // (`the_private_key_file_is_born_unreadable_for_anyone_else`), igual que la
    // de `restrict_to_owner` que usa la memoria entre sesiones.
}
