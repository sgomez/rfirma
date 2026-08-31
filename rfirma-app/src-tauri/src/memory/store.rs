//! El soporte de las dos memorias: un fichero JSON versionado que se escribe
//! **atómicamente** y que, cuando no se entiende, se aparta en vez de matar la
//! aplicación (ADR-0010).
//!
//! Tres reglas, y las tres son del ADR:
//!
//! - **Escritura atómica**: temporal y `rename`. Un fichero a medio escribir es
//!   una configuración que reaparece mutilada en el siguiente arranque.
//! - **`"version": 1` en los dos ficheros.** Se lee **antes** de deserializar,
//!   sobre el JSON en crudo: un fichero de una versión futura no se interpreta
//!   con las reglas de esta, se aparta.
//! - **Si no parsea o la versión es desconocida, se renombra a `.bak`** y se
//!   arranca con los valores por omisión, avisando **una vez**. Una preferencia
//!   corrupta no puede impedir firmar, así que esto no es un error: es una
//!   [`Recovery`] que viaja junto al valor.
//!
//! Que **no haya fichero** no es nada de lo anterior: es el primer arranque, y
//! da los valores por omisión sin aviso ninguno.

use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::error::{MemoryError, Situation};

/// La versión del formato de los dos ficheros. Sube cuando un cambio deje de
/// poder leerse con las reglas de la anterior; entonces esto necesitará una
/// migración, y hasta que exista un fichero de la versión vieja se aparta.
pub const FORMAT_VERSION: u64 = 1;

const VERSION_KEY: &str = "version";

/// Por qué se apartó lo que había guardado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Damage {
    /// El fichero no es JSON, o no es el JSON que este tipo describe.
    Unparsable(String),
    /// El JSON está bien pero declara una versión que esta rFirma no conoce
    /// —o no declara ninguna—. Casi siempre es una rFirma más nueva que ya
    /// escribió ahí.
    UnknownVersion(Option<u64>),
}

/// Lo que había guardado no se pudo usar y se apartó. Se avisa **una vez**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovery {
    backup: PathBuf,
    damage: Damage,
}

impl Recovery {
    /// Dónde quedó lo que había, por si alguien quiere mirarlo.
    pub fn backup(&self) -> &Path {
        &self.backup
    }

    /// Qué le pasaba.
    pub fn damage(&self) -> &Damage {
        &self.damage
    }
}

/// Lo leído, y el aviso si hubo que apartar algo para poder leerlo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded<T> {
    value: T,
    recovery: Option<Recovery>,
}

impl<T> Loaded<T> {
    /// El valor, siempre. En el peor caso, el de por omisión.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// El valor, para quedárselo.
    pub fn into_value(self) -> T {
        self.value
    }

    /// El aviso, si hubo que apartar lo que había.
    pub fn recovery(&self) -> Option<&Recovery> {
        self.recovery.as_ref()
    }
}

/// Un fichero JSON versionado que guarda un `T`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonFile<T> {
    path: PathBuf,
    kind: PhantomData<fn() -> T>,
}

impl<T> JsonFile<T> {
    /// El fichero que vive en `path`.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: PhantomData,
        }
    }

    /// El fichero que respalda esta memoria.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Olvida lo guardado borrando el soporte.
    ///
    /// Es lo que hace «Recordar mi actividad» al apagarse: conservar el fichero
    /// mientras la preferencia dice que no se recuerda nada incumple lo que
    /// promete el rótulo (ADR-0010). Un soporte que ya no estaba no es un
    /// fallo.
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
    /// Lee lo guardado, o los valores por omisión.
    ///
    /// Solo devuelve `Err` cuando el fichero **está y no se deja leer**. Que no
    /// esté, que no parsee o que traiga otra versión no es un fallo: es un
    /// arranque con los valores por omisión, con aviso en los dos últimos
    /// casos.
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
                recovery: Some(self.set_aside(damage)?),
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

    /// Aparta lo que no se ha podido usar. El `.bak` es **uno**: el interesante
    /// es el último, y guardar historia de ficheros rotos no ayuda a nadie.
    fn set_aside(&self, damage: Damage) -> Result<Recovery, MemoryError> {
        let backup = self.backup_path();
        fs::rename(&self.path, &backup)
            .map_err(|error| MemoryError::about(Situation::Unwritable, &backup, &error))?;
        Ok(Recovery { backup, damage })
    }
}

impl<T: Serialize> JsonFile<T> {
    /// Escribe el valor, sustituyendo lo que hubiera.
    ///
    /// Temporal y `rename` (ADR-0010): mientras el `rename` no ocurre, lo que
    /// hay en disco sigue siendo lo anterior, entero.
    pub fn save(&self, value: &T) -> Result<(), MemoryError> {
        let document = self.versioned(value)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| MemoryError::about(Situation::Unwritable, parent, &error))?;
        }
        let mut temporary = self.path.as_os_str().to_owned();
        temporary.push(".tmp");
        let temporary = PathBuf::from(temporary);
        fs::write(&temporary, &document)
            .map_err(|error| MemoryError::about(Situation::Unwritable, &temporary, &error))?;
        fs::rename(&temporary, &self.path).map_err(|error| {
            // El `rename` que falla deja el temporal escrito; barrerlo es parte
            // de fallar sin dejar rastro.
            let _ = fs::remove_file(&temporary);
            MemoryError::about(Situation::Unwritable, &self.path, &error)
        })
    }

    /// El JSON del valor con `"version"` metido dentro.
    ///
    /// La versión se pone aquí y no como un campo del tipo para que ningún `T`
    /// pueda escribir un número distinto del que este módulo sabe leer.
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
mod tests {
    use super::*;
    use serde::Deserialize;

    /// **Grada A**: un directorio temporal, sin token, sin librería nativa y
    /// sin red.
    #[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct Remembered {
        answer: u32,
    }

    fn a_file(directory: &Path) -> JsonFile<Remembered> {
        JsonFile::at(directory.join("rfirma/config.json"))
    }

    #[test]
    fn a_support_that_is_not_there_yet_gives_the_defaults_without_a_notice() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());

        let loaded = file.load().expect("deberia leerse");

        assert_eq!(loaded.value(), &Remembered::default());
        assert!(loaded.recovery().is_none());
    }

    #[test]
    fn what_is_saved_comes_back_and_carries_the_format_version() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());

        file.save(&Remembered { answer: 42 })
            .expect("deberia escribirse");

        assert_eq!(
            file.load().expect("deberia leerse").into_value(),
            Remembered { answer: 42 }
        );
        let written: Value =
            serde_json::from_slice(&fs::read(file.path()).expect("deberia leerse el fichero"))
                .expect("deberia ser JSON");
        assert_eq!(written["version"], Value::from(FORMAT_VERSION));
    }

    #[test]
    fn a_corrupt_support_is_set_aside_as_bak_and_the_application_still_starts() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());
        fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
            .expect("deberia crearse");
        fs::write(file.path(), b"{esto no es JSON").expect("deberia escribirse");

        let loaded = file
            .load()
            .expect("una preferencia corrupta no puede ser un fallo");

        assert_eq!(loaded.value(), &Remembered::default());
        let recovery = loaded.recovery().expect("deberia avisar una vez");
        assert!(matches!(recovery.damage(), Damage::Unparsable(_)));
        assert_eq!(
            recovery.backup(),
            directory.path().join("rfirma/config.json.bak")
        );
        assert!(
            recovery.backup().exists(),
            "lo que habia se conserva en el .bak"
        );
        assert!(
            !file.path().exists(),
            "el fichero roto ya no esta en su sitio"
        );
    }

    #[test]
    fn a_support_from_an_unknown_version_is_set_aside_instead_of_interpreted() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());
        fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
            .expect("deberia crearse");
        fs::write(file.path(), br#"{"version": 99, "answer": 7}"#).expect("deberia escribirse");

        let loaded = file
            .load()
            .expect("una version desconocida no puede ser un fallo");

        assert_eq!(loaded.value(), &Remembered::default());
        assert_eq!(
            loaded.recovery().map(Recovery::damage),
            Some(&Damage::UnknownVersion(Some(99)))
        );
    }

    #[test]
    fn a_support_without_a_version_is_set_aside_too() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());
        fs::create_dir_all(file.path().parent().expect("deberia tener padre"))
            .expect("deberia crearse");
        fs::write(file.path(), br#"{"answer": 7}"#).expect("deberia escribirse");

        let loaded = file.load().expect("deberia leerse");

        assert_eq!(
            loaded.recovery().map(Recovery::damage),
            Some(&Damage::UnknownVersion(None))
        );
    }

    #[test]
    fn a_failed_write_leaves_the_previous_content_intact_and_no_temporary_behind() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());
        file.save(&Remembered { answer: 1 })
            .expect("deberia escribirse");
        // Un directorio en el sitio del fichero: `rename` no puede con el.
        let taken = directory.path().join("rfirma/otro.json");
        fs::create_dir(&taken).expect("deberia crearse el directorio");
        let blocked: JsonFile<Remembered> = JsonFile::at(&taken);

        let error = blocked
            .save(&Remembered { answer: 2 })
            .expect_err("deberia fallar al escribir");

        assert_eq!(error.situation(), Situation::Unwritable);
        assert!(!directory.path().join("rfirma/otro.json.tmp").exists());
        assert_eq!(
            file.load().expect("deberia leerse").into_value(),
            Remembered { answer: 1 }
        );
    }

    #[test]
    fn a_support_that_exists_but_cannot_be_read_is_a_failure_and_not_the_defaults() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        // Un directorio en el sitio del fichero: existe, pero `read` no puede.
        let taken = directory.path().join("config.json");
        fs::create_dir(&taken).expect("deberia crearse el directorio");

        let error = JsonFile::<Remembered>::at(&taken)
            .load()
            .expect_err("un soporte ilegible no puede pasar por primer arranque");

        assert_eq!(error.situation(), Situation::Unreadable);
        assert!(error.detail().contains("config.json"));
    }

    #[test]
    fn erasing_removes_the_support_and_does_not_mind_it_being_gone_already() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let file = a_file(directory.path());
        file.save(&Remembered { answer: 3 })
            .expect("deberia escribirse");

        file.erase().expect("deberia borrarse");

        assert!(!file.path().exists());
        file.erase()
            .expect("borrar lo que ya no esta no es un fallo");
    }
}
