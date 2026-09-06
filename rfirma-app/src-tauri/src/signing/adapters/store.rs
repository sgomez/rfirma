//! Soporte en disco de las dos memorias con escritura atómica y versión de formato (ADR-0010).

use std::fs;
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::signing::adapters::memory_error::{MemoryError, Situation};

/// Versión del formato de los ficheros de memoria.
pub const FORMAT_VERSION: u64 = 1;

const VERSION_KEY: &str = "version";

/// Causa por la que se apartó lo que había guardado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Damage {
    /// El fichero no es JSON o no coincide con la estructura esperada.
    Unparsable(String),
    /// Versión declarada desconocida o ausente.
    UnknownVersion(Option<u64>),
}

/// Registro de contenido corrupto o desconocido que hubo que apartar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovery {
    backup: Option<PathBuf>,
    damage: Damage,
}

impl Recovery {
    /// Ruta del fichero de respaldo si pudo apartarse (ADR-0010).
    pub fn backup(&self) -> Option<&Path> {
        self.backup.as_deref()
    }

    /// Causa del descarte.
    pub fn damage(&self) -> &Damage {
        &self.damage
    }
}

/// Contenido leído junto con el eventual aviso de recuperación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded<T> {
    value: T,
    recovery: Option<Recovery>,
}

impl<T> Loaded<T> {
    /// Referencia al valor cargado o por omisión.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consume el envoltorio y devuelve el valor.
    pub fn into_value(self) -> T {
        self.value
    }

    /// Aviso de recuperación si el contenido previo tuvo que apartarse.
    pub fn recovery(&self) -> Option<&Recovery> {
        self.recovery.as_ref()
    }
}

/// Fichero JSON versionado con escritura atómica para un tipo `T` (ADR-0010).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonFile<T> {
    path: PathBuf,
    kind: PhantomData<fn() -> T>,
}

impl<T> JsonFile<T> {
    /// Crea una referencia al fichero en la ruta indicada.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: PhantomData,
        }
    }

    /// Ruta al fichero en disco.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Elimina el fichero en disco si existe (ADR-0010).
    pub fn erase(&self) -> Result<(), MemoryError> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(MemoryError::about(
                Situation::Unwritable,
                &self.path,
                &error,
            )),
        }
    }

    fn backup_path(&self) -> PathBuf {
        let mut name = self.path.as_os_str().to_owned();
        name.push(".bak");
        PathBuf::from(name)
    }
}

impl<T: DeserializeOwned + Default> JsonFile<T> {
    /// Carga el valor guardado o devuelve el valor por omisión si no existe o fue descartado.
    pub fn load(&self) -> Result<Loaded<T>, MemoryError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Loaded {
                    value: T::default(),
                    recovery: None,
                })
            }
            Err(error) => {
                return Err(MemoryError::about(
                    Situation::Unreadable,
                    &self.path,
                    &error,
                ))
            }
        };

        match self.interpret(&bytes) {
            Ok(value) => Ok(Loaded {
                value,
                recovery: None,
            }),
            Err(damage) => Ok(Loaded {
                value: T::default(),
                recovery: Some(self.set_aside(damage)),
            }),
        }
    }

    fn interpret(&self, bytes: &[u8]) -> Result<T, Damage> {
        let document: Value =
            serde_json::from_slice(bytes).map_err(|error| Damage::Unparsable(error.to_string()))?;
        let declared = document.get(VERSION_KEY).and_then(Value::as_u64);
        if declared != Some(FORMAT_VERSION) {
            return Err(Damage::UnknownVersion(declared));
        }
        serde_json::from_value(document).map_err(|error| Damage::Unparsable(error.to_string()))
    }

    fn set_aside(&self, damage: Damage) -> Recovery {
        let candidate = self.backup_path();
        let backup = fs::rename(&self.path, &candidate).ok().map(|()| candidate);
        Recovery { backup, damage }
    }
}

impl<T: Serialize> JsonFile<T> {
    /// Escribe atómicamente el valor serializado en disco (ADR-0010).
    pub fn save(&self, value: &T) -> Result<(), MemoryError> {
        let document = self.versioned(value)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| MemoryError::about(Situation::Unwritable, parent, &error))?;
            crate::desktop::adapters::paths::restrict_to_owner(parent)
                .map_err(|error| MemoryError::about(Situation::Unwritable, parent, &error))?;
        }
        let mut temporary = self.path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        Self::write_and_sync(&temporary, &document)
            .map_err(|error| MemoryError::about(Situation::Unwritable, &temporary, &error))?;
        fs::rename(&temporary, &self.path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            MemoryError::about(Situation::Unwritable, &self.path, &error)
        })
    }

    fn write_and_sync(temporary: &Path, document: &[u8]) -> std::io::Result<()> {
        let mut file = fs::File::create(temporary)?;
        crate::desktop::adapters::paths::restrict_to_owner(temporary)?;
        file.write_all(document)?;
        file.sync_all()
    }

    fn versioned(&self, value: &T) -> Result<Vec<u8>, MemoryError> {
        let unwritable = |detail: String| {
            MemoryError::new(
                Situation::Unwritable,
                format!("{}: {detail}", self.path.display()),
            )
        };
        let serialized =
            serde_json::to_value(value).map_err(|error| unwritable(error.to_string()))?;
        let Value::Object(mut fields) = serialized else {
            return Err(unwritable("lo guardado no es un objeto JSON".to_owned()));
        };
        fields.insert(VERSION_KEY.to_owned(), Value::from(FORMAT_VERSION));
        let mut bytes = serde_json::to_vec_pretty(&Value::Object(fields))
            .map_err(|error| unwritable(error.to_string()))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests;
