//! Las tres rutas de la memoria entre sesiones, y **el único sitio del código
//! con un `cfg!` de sistema operativo** (ID-35, ADR-0010).
//!
//! Las rutas no son la decisión: la decisión es que hay **tres nombres** —
//! [`Paths::config_file`], [`Paths::state_file`] y [`Paths::rubric_path`]— y
//! que el resto de la aplicación no sabe qué sistema hay debajo. Añadir macOS
//! o Windows toca este fichero y ninguno más; si aparece un segundo `cfg!` de
//! sistema operativo en el repositorio, algo se ha hecho mal, y hay una prueba
//! que lo comprueba (`tests/single_cfg_os_site.rs`).
//!
//! La separación entre **configuración** y **estado** es la razón de que esto
//! no sea una sola carpeta: en Windows el estado no debe viajar en un perfil
//! móvil y la configuración sí, y en Linux `XDG_STATE_HOME` existe justo para
//! eso. macOS no distingue las dos cosas, y ahí la separación se colapsa a dos
//! ficheros en el mismo directorio.
//!
//! El hito v0.1 es solo Linux; las otras dos columnas se escriben ahora porque
//! el momento de saberlo es antes de escribir este fichero, no después.
//!
//! Nada aquí toca el disco: resolver una ruta no crea un directorio. Quien
//! escribe es [`crate::memory`], y crea el directorio en ese momento.

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

/// El nombre de la aplicación dentro de cada directorio del sistema.
pub const APPLICATION_DIRECTORY: &str = "rfirma";

const CONFIG_FILE: &str = "config.json";
const STATE_FILE: &str = "state.json";
/// La rúbrica se guarda ya normalizada a JPEG (ADR-0012), y la extensión lo
/// dice: lo que hay ahí dentro nunca es el PNG que eligió el usuario.
const RUBRIC_FILE: &str = "rubric.jpg";

/// El sistema operativo, reducido a lo único que cambia: dónde van las tres
/// rutas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// `XDG_CONFIG_HOME`, `XDG_STATE_HOME` y `XDG_DATA_HOME`.
    Linux,
    /// `%APPDATA%` para lo que puede viajar y `%LOCALAPPDATA%` para lo que no.
    Windows,
    /// `~/Library/Application Support`, el mismo directorio para las tres.
    MacOs,
}

impl Platform {
    /// El sistema sobre el que se ha compilado. **Este es el `cfg!`**: el resto
    /// del módulo es código normal que recibe una [`Platform`], y por eso las
    /// tres columnas de la tabla del ADR-0010 se prueban en cualquier máquina.
    pub const CURRENT: Self = if cfg!(target_os = "windows") {
        Self::Windows
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Linux
    };
}

/// El entorno no dice dónde vive el usuario.
///
/// No se inventa un directorio ni se cae a `.`: escribir la configuración en el
/// directorio de trabajo es peor que no escribirla, porque nadie la vuelve a
/// encontrar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeUnknown {
    variable: &'static str,
}

impl HomeUnknown {
    /// La variable de entorno que hacía falta y no estaba.
    pub fn variable(&self) -> &'static str {
        self.variable
    }
}

impl fmt::Display for HomeUnknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "falta la variable de entorno {}", self.variable)
    }
}

impl std::error::Error for HomeUnknown {}

/// Los tres directorios ya resueltos, con `rfirma/` incluido.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    config_dir: PathBuf,
    state_dir: PathBuf,
    data_dir: PathBuf,
}

impl Paths {
    /// Las rutas del sistema sobre el que corre la aplicación.
    pub fn from_environment() -> Result<Self, HomeUnknown> {
        Self::resolve(Platform::CURRENT, &|name| std::env::var_os(name))
    }

    /// Las rutas de `platform`, leyendo el entorno con `environment`.
    ///
    /// Es pública para que las pruebas cubran las tres columnas del ADR-0010
    /// sin tocar el entorno del proceso —cambiarlo es global y las pruebas
    /// corren en hilos—, y para que un empaquetado pueda resolverlas contra
    /// otro entorno si algún día hace falta.
    pub fn resolve(
        platform: Platform,
        environment: &dyn Fn(&str) -> Option<OsString>,
    ) -> Result<Self, HomeUnknown> {
        let directories = match platform {
            Platform::Linux => linux_directories(environment)?,
            Platform::Windows => windows_directories(environment)?,
            Platform::MacOs => macos_directories(environment)?,
        };
        let [config_dir, state_dir, data_dir] = directories;
        Ok(Self {
            config_dir: config_dir.join(APPLICATION_DIRECTORY),
            state_dir: state_dir.join(APPLICATION_DIRECTORY),
            data_dir: data_dir.join(APPLICATION_DIRECTORY),
        })
    }

    /// Las tres rutas colgando de `root`, para pruebas y para arrancar contra
    /// un directorio temporal.
    ///
    /// Reproduce la separación de Linux —`config/`, `state/` y `data/`— porque
    /// una prueba que escribe las dos memorias en el mismo fichero no probaría
    /// que están partidas.
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            config_dir: root.join("config").join(APPLICATION_DIRECTORY),
            state_dir: root.join("state").join(APPLICATION_DIRECTORY),
            data_dir: root.join("data").join(APPLICATION_DIRECTORY),
        }
    }

    /// El fichero de **configuración**: lo que el usuario elige.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    /// El fichero de **estado**: lo que la aplicación acumula sola.
    pub fn state_file(&self) -> PathBuf {
        self.state_dir.join(STATE_FILE)
    }

    /// La copia de la rúbrica. Es la copia, nunca la ruta del original
    /// (ID-33): quien la escribe es [`crate::rubric::RubricStore`].
    pub fn rubric_path(&self) -> PathBuf {
        self.data_dir.join(RUBRIC_FILE)
    }
}

/// `$XDG_*_HOME` si está y es absoluta, y si no `$HOME/<relativa>`.
///
/// La especificación XDG manda **ignorar** un valor relativo, no tratarlo como
/// relativo a nada: una `XDG_CONFIG_HOME=.config` heredada de un script no
/// puede acabar escribiendo en el directorio de trabajo de turno.
fn xdg_directory(
    environment: &dyn Fn(&str) -> Option<OsString>,
    variable: &'static str,
    fallback: &str,
) -> Result<PathBuf, HomeUnknown> {
    if let Some(value) = environment(variable) {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            return Ok(path);
        }
    }
    Ok(home(environment, "HOME")?.join(fallback))
}

fn home(
    environment: &dyn Fn(&str) -> Option<OsString>,
    variable: &'static str,
) -> Result<PathBuf, HomeUnknown> {
    environment(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(HomeUnknown { variable })
}

fn linux_directories(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<[PathBuf; 3], HomeUnknown> {
    Ok([
        xdg_directory(environment, "XDG_CONFIG_HOME", ".config")?,
        xdg_directory(environment, "XDG_STATE_HOME", ".local/state")?,
        xdg_directory(environment, "XDG_DATA_HOME", ".local/share")?,
    ])
}

fn windows_directories(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<[PathBuf; 3], HomeUnknown> {
    let roaming = home(environment, "APPDATA")?;
    // `%LOCALAPPDATA%` no impide otra cosa que una lista de rutas locales viaje
    // por la red, que es exactamente lo que el estado es.
    let local = home(environment, "LOCALAPPDATA")?;
    Ok([roaming.clone(), local, roaming])
}

fn macos_directories(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<[PathBuf; 3], HomeUnknown> {
    let support = home(environment, "HOME")?
        .join("Library")
        .join("Application Support");
    Ok([support.clone(), support.clone(), support])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// **Grada A**: no lee el entorno del proceso ni toca el disco.
    fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), OsString::from(*value)))
            .collect();
        move |name: &str| map.get(name).cloned()
    }

    fn resolve(platform: Platform, pairs: &[(&str, &str)]) -> Paths {
        Paths::resolve(platform, &environment(pairs)).expect("deberia resolverse")
    }

    #[test]
    fn linux_splits_configuration_and_state_across_two_xdg_directories() {
        let paths = resolve(
            Platform::Linux,
            &[
                ("HOME", "/home/quien"),
                ("XDG_CONFIG_HOME", "/home/quien/.config"),
                ("XDG_STATE_HOME", "/home/quien/.local/state"),
                ("XDG_DATA_HOME", "/home/quien/.local/share"),
            ],
        );

        assert_eq!(
            paths.config_file(),
            PathBuf::from("/home/quien/.config/rfirma/config.json")
        );
        assert_eq!(
            paths.state_file(),
            PathBuf::from("/home/quien/.local/state/rfirma/state.json")
        );
        assert_eq!(
            paths.rubric_path(),
            PathBuf::from("/home/quien/.local/share/rfirma/rubric.jpg")
        );
    }

    #[test]
    fn linux_falls_back_to_the_xdg_defaults_under_home() {
        let paths = resolve(Platform::Linux, &[("HOME", "/home/quien")]);

        assert_eq!(
            paths.config_file(),
            PathBuf::from("/home/quien/.config/rfirma/config.json")
        );
        assert_eq!(
            paths.state_file(),
            PathBuf::from("/home/quien/.local/state/rfirma/state.json")
        );
    }

    #[test]
    fn a_relative_xdg_variable_is_ignored_instead_of_writing_next_to_the_cwd() {
        let paths = resolve(
            Platform::Linux,
            &[("HOME", "/home/quien"), ("XDG_CONFIG_HOME", ".config")],
        );

        assert_eq!(
            paths.config_file(),
            PathBuf::from("/home/quien/.config/rfirma/config.json")
        );
    }

    #[test]
    fn windows_keeps_the_state_out_of_the_roaming_profile() {
        let paths = resolve(
            Platform::Windows,
            &[
                ("APPDATA", r"C:\Users\quien\AppData\Roaming"),
                ("LOCALAPPDATA", r"C:\Users\quien\AppData\Local"),
            ],
        );

        assert_eq!(
            paths.config_file(),
            PathBuf::from(r"C:\Users\quien\AppData\Roaming").join("rfirma/config.json")
        );
        assert_eq!(
            paths.state_file(),
            PathBuf::from(r"C:\Users\quien\AppData\Local").join("rfirma/state.json")
        );
        assert_eq!(
            paths.rubric_path(),
            PathBuf::from(r"C:\Users\quien\AppData\Roaming").join("rfirma/rubric.jpg")
        );
    }

    #[test]
    fn macos_collapses_the_split_into_two_files_in_one_directory() {
        let paths = resolve(Platform::MacOs, &[("HOME", "/Users/quien")]);

        assert_eq!(
            paths.config_file().parent(),
            paths.state_file().parent(),
            "en macOS los dos ficheros comparten directorio"
        );
        assert_ne!(paths.config_file(), paths.state_file());
        assert_eq!(
            paths.config_file(),
            PathBuf::from("/Users/quien/Library/Application Support/rfirma/config.json")
        );
    }

    #[test]
    fn an_environment_without_a_home_is_a_failure_naming_the_variable() {
        let error = Paths::resolve(Platform::Linux, &environment(&[]))
            .expect_err("sin HOME no deberia resolverse");

        assert_eq!(error.variable(), "HOME");
        assert!(error.to_string().contains("HOME"));
    }

    #[test]
    fn windows_without_the_local_profile_is_a_failure_naming_it() {
        let error = Paths::resolve(
            Platform::Windows,
            &environment(&[("APPDATA", r"C:\Users\quien\AppData\Roaming")]),
        )
        .expect_err("sin LOCALAPPDATA no deberia resolverse");

        assert_eq!(error.variable(), "LOCALAPPDATA");
    }

    #[test]
    fn the_three_memories_never_share_a_file() {
        for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
            let paths = resolve(
                platform,
                &[
                    ("HOME", "/home/quien"),
                    ("APPDATA", "/roaming"),
                    ("LOCALAPPDATA", "/local"),
                ],
            );
            let files = [paths.config_file(), paths.state_file(), paths.rubric_path()];
            for (index, file) in files.iter().enumerate() {
                assert!(
                    !files[index + 1..].contains(file),
                    "{platform:?} repite {}",
                    file.display()
                );
            }
        }
    }

    #[test]
    fn resolving_a_path_does_not_create_anything_on_disk() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let paths = Paths::under(directory.path());

        assert!(!paths.config_file().exists());
        assert!(!paths.state_file().exists());
        assert!(!paths.rubric_path().exists());
    }

    #[test]
    fn the_test_root_keeps_configuration_and_state_apart() {
        let paths = Paths::under("/tmp/prueba");

        assert_ne!(paths.config_file().parent(), paths.state_file().parent());
    }
}
