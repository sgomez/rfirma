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
mod tests {
    use super::*;
    use std::collections::HashMap;

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
    fn the_configuration_home_has_no_application_directory_behind_it() {
        let home = xdg_config_home(&environment(&[
            ("HOME", "/home/quien"),
            ("XDG_CONFIG_HOME", "/home/quien/.config"),
        ]))
        .expect("deberia resolverse");

        assert_eq!(home, PathBuf::from("/home/quien/.config"));
    }

    #[test]
    fn without_the_variable_the_configuration_home_falls_back_under_home() {
        let home =
            xdg_config_home(&environment(&[("HOME", "/home/quien")])).expect("deberia resolverse");

        assert_eq!(home, PathBuf::from("/home/quien/.config"));
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

    #[test]
    fn the_documents_folder_follows_the_xdg_variable_when_the_system_localises_it() {
        let documents = documents_folder_of(
            Platform::Linux,
            &environment(&[
                ("HOME", "/home/quien"),
                ("XDG_DOCUMENTS_DIR", "/home/quien/Documentos"),
            ]),
        )
        .expect("deberia resolverse");

        assert_eq!(documents, PathBuf::from("/home/quien/Documentos"));
    }

    #[test]
    fn without_the_xdg_variable_the_documents_folder_is_the_english_default() {
        let documents =
            documents_folder_of(Platform::Linux, &environment(&[("HOME", "/home/quien")]))
                .expect("deberia resolverse");

        assert_eq!(documents, PathBuf::from("/home/quien/Documents"));
    }

    #[test]
    fn the_other_two_systems_hang_the_documents_folder_off_their_own_profile() {
        let windows = documents_folder_of(
            Platform::Windows,
            &environment(&[("USERPROFILE", r"C:\Users\quien")]),
        )
        .expect("deberia resolverse");
        let macos = documents_folder_of(Platform::MacOs, &environment(&[("HOME", "/Users/quien")]))
            .expect("deberia resolverse");

        assert_eq!(windows, PathBuf::from(r"C:\Users\quien").join("Documents"));
        assert_eq!(macos, PathBuf::from("/Users/quien/Documents"));
    }

    #[test]
    fn resolving_the_documents_folder_does_not_create_it() {
        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let home = directory.path().join("quien");

        let documents = documents_folder_of(
            Platform::Linux,
            &environment(&[("HOME", &home.to_string_lossy())]),
        )
        .expect("deberia resolverse");

        assert!(
            !documents.exists(),
            "resolver una ruta no toca el disco, y la de destino no se crea nunca (ADR-0011)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn restricting_leaves_the_directory_and_the_file_only_for_their_owner() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let inside = directory.path().join("rfirma");
        std::fs::create_dir(&inside).expect("deberia crearse");
        let file = inside.join("state.json");
        std::fs::write(&file, b"{}").expect("deberia escribirse");

        restrict_to_owner(&inside).expect("deberia poder restringirse");
        restrict_to_owner(&file).expect("deberia poder restringirse");

        let mode = |path: &Path| {
            std::fs::metadata(path)
                .expect("deberia leerse")
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!(mode(&inside), 0o700);
        assert_eq!(mode(&file), 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn the_private_key_file_is_born_unreadable_for_anyone_else() {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("deberia haber directorio temporal");
        let key = directory.path().join("local-ca.key.pem");

        let mut file = create_owner_only_file(&key).expect("deberia crearse");
        file.write_all(b"-----BEGIN PRIVATE KEY-----")
            .expect("deberia escribirse");

        assert_eq!(
            std::fs::metadata(&key)
                .expect("deberia leerse")
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "el modo va en el `open`, no en un `chmod` posterior (ADR-0005)"
        );
    }

    #[test]
    fn the_local_ca_lives_in_the_data_directory_and_the_server_certificate_nowhere() {
        let paths = Paths::under("/tmp/raiz");

        assert_eq!(
            paths.local_ca_certificate_path(),
            PathBuf::from("/tmp/raiz/data/rfirma/local-ca.crt.pem")
        );
        assert_eq!(
            paths.local_ca_key_path(),
            PathBuf::from("/tmp/raiz/data/rfirma/local-ca.key.pem")
        );
    }
}
