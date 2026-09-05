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
//! El almacén **no decide nada**: no renueva, no reinstala y no decide cuándo
//! empieza ni cuándo acaba el solape. Solo sostiene **dos ranuras** —la CA que
//! sirve y la siguiente, que espera su turno—, lee lo que hay y escribe lo que
//! le den. Quién ocupa cada ranura y cuándo lo decide el caso de uso que
//! registra en los almacenes NSS.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::authority::LocalCa;
use super::error::{Situation, TlsError};
use crate::paths::{create_owner_only_file, restrict_to_owner, Paths};

/// **El par de ficheros de una CA local**: su certificado y su clave.
///
/// Es una ranura, no la CA: el almacén tiene dos, la que sirve y la siguiente.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaFiles {
    certificate_path: PathBuf,
    key_path: PathBuf,
}

impl CaFiles {
    /// La ranura que vive en esas dos rutas.
    pub fn new(certificate_path: impl Into<PathBuf>, key_path: impl Into<PathBuf>) -> Self {
        Self {
            certificate_path: certificate_path.into(),
            key_path: key_path.into(),
        }
    }

    /// La ruta del certificado, que es lo que se registra en los almacenes NSS.
    pub fn certificate_path(&self) -> &Path {
        &self.certificate_path
    }

    /// La CA local que hubiera en esta ranura, o `None` si está vacía.
    fn read(&self) -> Result<Option<LocalCa>, TlsError> {
        if !self.certificate_path.exists() || !self.key_path.exists() {
            return Ok(None);
        }
        let certificate = read_file(&self.certificate_path)?;
        let key = read_file(&self.key_path)?;
        LocalCa::from_pem(&certificate, &key).map(Some)
    }

    /// Guarda la CA local en esta ranura, sustituyendo la que hubiera.
    ///
    /// La clave se crea **ya** con el modo `0600` y no con un `chmod` posterior
    /// (ADR-0005): entre el `create` y el `chmod` la clave privada estaría
    /// legible para cualquier cuenta de la máquina.
    fn write(&self, ca: &LocalCa) -> Result<(), TlsError> {
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

    /// Vacía la ranura. Que ya estuviera vacía **no es un fallo**.
    fn empty(&self) -> Result<(), TlsError> {
        let unwritable = |path: &Path, error: std::io::Error| {
            TlsError::new(
                Situation::MaterialUnwritable,
                format!("{}: {error}", path.display()),
            )
        };
        for path in [&self.certificate_path, &self.key_path] {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(unwritable(path, error)),
            }
        }
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, TlsError> {
    std::fs::read(path).map_err(|error| {
        TlsError::new(
            Situation::MaterialDamaged,
            format!("{}: {error}", path.display()),
        )
    })
}

/// **Las dos ranuras de la CA local**: la que sirve y la siguiente.
///
/// Son dos y no una porque el solape del ID-224 exige que durante meses haya
/// **dos CA locales vivas**: la vigente, que es la que firma el certificado del
/// servidor local en cada arranque, y la siguiente, ya instalada en los
/// almacenes NSS y esperando a que la vigente caduque. Guardar la siguiente
/// encima de la vigente dejaría el solape sin efecto —el navegador que ya
/// estaba abierto recibiría un certificado firmado por una CA que no ha
/// cargado— y el trámite inmediatamente posterior a la renovación fallaría
/// igual que sin solape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalCaStore {
    serving: CaFiles,
    next: CaFiles,
}

impl LocalCaStore {
    /// El almacén con esas dos ranuras.
    pub fn new(serving: CaFiles, next: CaFiles) -> Self {
        Self { serving, next }
    }

    /// El almacén en el directorio de datos de la aplicación.
    pub fn of(paths: &Paths) -> Self {
        Self::new(
            CaFiles::new(paths.local_ca_certificate_path(), paths.local_ca_key_path()),
            CaFiles::new(
                paths.next_local_ca_certificate_path(),
                paths.next_local_ca_key_path(),
            ),
        )
    }

    /// La ruta del certificado de la CA que **sirve**.
    pub fn certificate_path(&self) -> &Path {
        self.serving.certificate_path()
    }

    /// La CA local que sirve, o `None` si todavía no hay ninguna.
    ///
    /// La ausencia **no es un fallo**: un `$HOME` sin CA local es el primer
    /// arranque. Lo que sí lo es —y se dice— es que los ficheros estén y no se
    /// entiendan.
    pub fn read(&self) -> Result<Option<LocalCa>, TlsError> {
        self.serving.read()
    }

    /// Guarda la CA que sirve, sustituyendo la que hubiera.
    pub fn write(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.serving.write(ca)
    }

    /// La CA local **siguiente** si ya está fabricada, o `None`.
    pub fn read_next(&self) -> Result<Option<LocalCa>, TlsError> {
        self.next.read()
    }

    /// Guarda la CA local siguiente **sin tocar la que sirve**.
    pub fn write_next(&self, ca: &LocalCa) -> Result<(), TlsError> {
        self.next.write(ca)
    }

    /// **La siguiente pasa a servir**: se copia a la ranura de la vigente y la
    /// suya se vacía.
    ///
    /// Es el final del solape, y la razón de que valga la pena: la CA que toma
    /// el relevo lleva meses instalada en los almacenes, así que el navegador
    /// ya confía en ella y no hay que reiniciar nada.
    pub fn promote_next(&self) -> Result<Option<LocalCa>, TlsError> {
        let Some(next) = self.next.read()? else {
            return Ok(None);
        };
        self.serving.write(&next)?;
        self.next.empty()?;
        Ok(Some(next))
    }

    /// Tira la CA local siguiente que hubiera. Se hace al fabricar una vigente
    /// nueva: la que esperaba turno ya no lo tendrá.
    pub fn forget_next(&self) -> Result<(), TlsError> {
        self.next.empty()
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

    /// **El solape, del lado del almacén**: la siguiente se guarda **al lado**
    /// de la vigente, no encima. Si la sustituyera, el certificado del servidor
    /// local pasaría a salir firmado por una CA que el navegador abierto no ha
    /// cargado, y el trámite siguiente fallaría igual que sin solape.
    #[test]
    fn the_next_local_ca_is_saved_beside_the_serving_one_and_not_over_it() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = store_in(directory.path());
        let serving = LocalCa::generate().expect("deberia generarse");
        let next = LocalCa::generate().expect("deberia generarse");

        store.write(&serving).expect("deberia guardarse la vigente");
        store
            .write_next(&next)
            .expect("deberia guardarse la siguiente");

        assert_eq!(
            store
                .read()
                .unwrap()
                .unwrap()
                .certificate()
                .to_pem()
                .unwrap(),
            serving.certificate().to_pem().unwrap(),
            "la que sirve sigue siendo la vigente"
        );
        assert_eq!(
            store
                .read_next()
                .unwrap()
                .unwrap()
                .certificate()
                .to_pem()
                .unwrap(),
            next.certificate().to_pem().unwrap()
        );
    }

    /// **El relevo**: la siguiente pasa a servir y su ranura queda vacía, así
    /// que el arranque de después ya no ve ninguna esperando turno.
    #[test]
    fn the_next_local_ca_takes_over_and_leaves_its_slot_empty() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = store_in(directory.path());
        store
            .write(&LocalCa::generate().expect("deberia generarse"))
            .expect("deberia guardarse");
        let next = LocalCa::generate().expect("deberia generarse");
        store.write_next(&next).expect("deberia guardarse");

        let promoted = store
            .promote_next()
            .expect("deberia poder relevarse")
            .expect("habia una siguiente esperando");

        assert_eq!(
            promoted.certificate().to_pem().unwrap(),
            next.certificate().to_pem().unwrap()
        );
        assert_eq!(
            store
                .read()
                .unwrap()
                .unwrap()
                .certificate()
                .to_pem()
                .unwrap(),
            next.certificate().to_pem().unwrap()
        );
        assert!(store.read_next().unwrap().is_none());
    }

    /// Sin ninguna siguiente esperando, el relevo no es un fallo: es un `None`.
    #[test]
    fn a_takeover_without_a_next_local_ca_is_not_a_failure() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let store = store_in(directory.path());

        assert!(store.promote_next().expect("no es un fallo").is_none());
        assert!(store.forget_next().is_ok(), "tirar lo que no hay tampoco");
    }

    // El modo `0600` del fichero de la clave **no se comprueba aquí**: leerlo
    // pide `std::os::unix`, y el ID-35 deja ese conocimiento en un solo
    // fichero. La prueba vive junto a la puerta que lo pone, en `paths.rs`
    // (`the_private_key_file_is_born_unreadable_for_anyone_else`), igual que la
    // de `restrict_to_owner` que usa la memoria entre sesiones.
}
