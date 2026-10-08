//! La tarjeta de una prueba: un directorio propio con su copia del módulo, su contador y su registro de llamadas.

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::card::{CALL_LOG, TRIES_LEFT};

/// Una tarjeta aislada: cada copia del módulo es una instancia aparte, aunque viva en el mismo proceso.
pub struct FakeCard {
    dir: TempDir,
}

impl FakeCard {
    /// El PIN que acepta la tarjeta.
    pub const PIN: &'static str = crate::card::PIN;

    /// Una tarjeta sana, con los tres intentos.
    pub fn new() -> io::Result<Self> {
        Self::with_tries_left(crate::card::MAX_TRIES)
    }

    /// Una tarjeta bloqueada, que rechaza hasta el PIN correcto.
    pub fn locked() -> io::Result<Self> {
        Self::with_tries_left(0)
    }

    /// Una tarjeta a la que le quedan `tries` intentos.
    pub fn with_tries_left(tries: u8) -> io::Result<Self> {
        let dir = tempfile::Builder::new().prefix("fake-pkcs11-").tempdir()?;
        fs::write(dir.path().join(TRIES_LEFT), tries.to_string())?;
        fs::copy(built_module()?, dir.path().join(module_file_name()))?;
        Ok(Self { dir })
    }

    /// La ruta del módulo que se le da a `cryptoki` o a un almacén de clase tarjeta.
    pub fn module(&self) -> PathBuf {
        self.dir.path().join(module_file_name())
    }

    /// Los intentos que le quedan a la tarjeta, como los guarda ella.
    pub fn tries_left(&self) -> u8 {
        crate::card::read_tries_left(self.dir.path())
    }

    /// Cada llamada que ha recibido el módulo, una por línea: `C_Login CKU_USER "…" -> CKR_OK`.
    pub fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.path().join(CALL_LOG))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// Las llamadas a una función concreta, por su nombre PKCS#11.
    pub fn calls_to(&self, function: &str) -> Vec<String> {
        let prefix = format!("{function} ");
        self.calls()
            .into_iter()
            .filter(|call| call.starts_with(&prefix))
            .collect()
    }
}

fn module_file_name() -> String {
    format!("{DLL_PREFIX}fake_pkcs11{DLL_SUFFIX}")
}

/// El módulo que ha dejado cargo junto al ejecutable de la prueba, o un directorio más arriba.
fn built_module() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    executable
        .ancestors()
        .skip(1)
        .take(2)
        .map(|dir| dir.join(module_file_name()))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| not_built(&executable))
}

fn not_built(executable: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "no hay {} junto a {}: compila fake-pkcs11 en el mismo árbol",
            module_file_name(),
            executable.display()
        ),
    )
}
