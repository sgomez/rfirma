//! Resolución de rutas del sistema y permisos de fichero entre sesiones (ADR-0010).

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

/// Nombre de la aplicación dentro de cada directorio del sistema.
pub const APPLICATION_DIRECTORY: &str = "rfirma";

const CONFIG_FILE: &str = "config.json";
const STATE_FILE: &str = "state.json";
/// Nombre del fichero de rúbrica normalizada (ADR-0012).
const RUBRIC_FILE: &str = "rubric.jpg";
/// Directorio para almacenes NSS de certificados importados.
const INSTALLED_CERTIFICATES_DIRECTORY: &str = "certificates";
/// Fichero del certificado de la CA local (ADR-0005).
const LOCAL_CA_CERTIFICATE_FILE: &str = "local-ca.crt.pem";
/// Fichero de clave privada de la CA local (ADR-0005).
const LOCAL_CA_KEY_FILE: &str = "local-ca.key.pem";
/// Fichero del certificado siguiente de la CA local (ADR-0005).
const NEXT_LOCAL_CA_CERTIFICATE_FILE: &str = "local-ca-next.crt.pem";
/// Fichero de clave privada siguiente de la CA local (ADR-0005).
const NEXT_LOCAL_CA_KEY_FILE: &str = "local-ca-next.key.pem";

/// Plataformas soportadas para la resolución de rutas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Entorno Linux basado en estándares XDG.
    Linux,
    /// Entorno Windows basado en perfiles de usuario.
    Windows,
    /// Entorno macOS basado en Application Support.
    MacOs,
}

impl Platform {
    /// Plataforma sobre la que se compila la aplicación (ADR-0010).
    pub const CURRENT: Self = if cfg!(target_os = "windows") {
        Self::Windows
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Linux
    };
}

/// Restringe los permisos de una ruta exclusivamente a su propietario.
pub fn restrict_to_owner(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        const OWNER_ONLY_DIRECTORY: u32 = 0o700;
        const OWNER_ONLY_FILE: u32 = 0o600;

        let mode = if path.metadata()?.is_dir() {
            OWNER_ONLY_DIRECTORY
        } else {
            OWNER_ONLY_FILE
        };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Error producido cuando no se puede determinar el directorio personal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HomeUnknown {
    variable: &'static str,
}

impl HomeUnknown {
    /// Variable de entorno requerida no encontrada.
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

/// Rutas resueltas de configuración, estado y datos de la aplicación.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    config_dir: PathBuf,
    state_dir: PathBuf,
    data_dir: PathBuf,
}

impl Paths {
    /// Resuelve las rutas a partir de las variables de entorno actuales.
    pub fn from_environment() -> Result<Self, HomeUnknown> {
        Self::resolve(Platform::CURRENT, &|name| std::env::var_os(name))
    }

    /// Resuelve las rutas para una plataforma dada usando una función de entorno (ADR-0010).
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

    /// Construye las rutas bajo un directorio raíz arbitrario.
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            config_dir: root.join("config").join(APPLICATION_DIRECTORY),
            state_dir: root.join("state").join(APPLICATION_DIRECTORY),
            data_dir: root.join("data").join(APPLICATION_DIRECTORY),
        }
    }

    /// Ruta del fichero de configuración de la aplicación.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    /// Ruta del fichero de estado de la aplicación.
    pub fn state_file(&self) -> PathBuf {
        self.state_dir.join(STATE_FILE)
    }

    /// Ruta de la copia local de la rúbrica (ADR-0012).
    pub fn rubric_path(&self) -> PathBuf {
        self.data_dir.join(RUBRIC_FILE)
    }

    /// Directorio de certificados instalados en almacenes NSS.
    pub fn installed_certificates_dir(&self) -> PathBuf {
        self.data_dir.join(INSTALLED_CERTIFICATES_DIRECTORY)
    }

    /// Ruta del certificado de la CA local (ADR-0005).
    pub fn local_ca_certificate_path(&self) -> PathBuf {
        self.data_dir.join(LOCAL_CA_CERTIFICATE_FILE)
    }

    /// Ruta de la clave privada de la CA local (ADR-0005).
    pub fn local_ca_key_path(&self) -> PathBuf {
        self.data_dir.join(LOCAL_CA_KEY_FILE)
    }

    /// Ruta del certificado siguiente de la CA local (ADR-0005).
    pub fn next_local_ca_certificate_path(&self) -> PathBuf {
        self.data_dir.join(NEXT_LOCAL_CA_CERTIFICATE_FILE)
    }

    /// Ruta de la clave privada siguiente de la CA local (ADR-0005).
    pub fn next_local_ca_key_path(&self) -> PathBuf {
        self.data_dir.join(NEXT_LOCAL_CA_KEY_FILE)
    }
}

/// Crea un fichero con permisos restringidos exclusivamente a su dueño desde su creación (ADR-0005).
pub fn create_owner_only_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;

        const OWNER_ONLY_FILE: u32 = 0o600;

        options.mode(OWNER_ONLY_FILE);
    }
    options.open(path)
}

/// Nombre por defecto del directorio de documentos.
const DOCUMENTS_DIRECTORY: &str = "Documents";

/// Directorio de documentos del usuario actual (ADR-0011).
pub fn documents_folder() -> Result<PathBuf, HomeUnknown> {
    documents_folder_of(Platform::CURRENT, &|name| std::env::var_os(name))
}

/// Resuelve el directorio de documentos para una plataforma dada (ADR-0011).
pub fn documents_folder_of(
    platform: Platform,
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, HomeUnknown> {
    match platform {
        Platform::Linux => xdg_directory(environment, "XDG_DOCUMENTS_DIR", DOCUMENTS_DIRECTORY),
        Platform::Windows => Ok(home(environment, "USERPROFILE")?.join(DOCUMENTS_DIRECTORY)),
        Platform::MacOs => Ok(home(environment, "HOME")?.join(DOCUMENTS_DIRECTORY)),
    }
}

/// Directorio base de configuración XDG del usuario.
pub fn xdg_config_home(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, HomeUnknown> {
    xdg_directory(environment, "XDG_CONFIG_HOME", ".config")
}

/// Resuelve un directorio XDG considerando rutas absolutas y respaldo en HOME.
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
mod tests;
