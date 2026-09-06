//! Colección y descubrimiento de almacenes PKCS#11 y perfiles NSS.

use std::path::{Path, PathBuf};

/// Rutas candidatas para módulos PKCS#11 estándar.
pub const CANDIDATE_MODULES: &[&str] = &[
    "/usr/lib/softhsm/libsofthsm2.so",
    "/usr/lib/x86_64-linux-gnu/softhsm/libsofthsm2.so",
];

/// Rutas candidatas para bibliotecas softoken de NSS.
pub const CANDIDATE_SOFTOKENS: &[&str] = &[
    "/usr/lib/x86_64-linux-gnu/libsoftokn3.so",
    "/usr/lib/x86_64-linux-gnu/nss/libsoftokn3.so",
    "/usr/lib64/libsoftokn3.so",
    "/usr/lib64/nss/libsoftokn3.so",
    "/usr/lib/libsoftokn3.so",
    "/usr/lib/nss/libsoftokn3.so",
];

/// Clasificación del tipo de almacén para presentación en la interfaz (ADR-0011).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StoreClass {
    /// Módulo PKCS#11 de tarjeta o token físico.
    Card,
    /// Perfil de usuario del navegador Firefox.
    Firefox,
    /// Almacén NSS compartido de la familia Chromium.
    Chrome,
    /// Base de datos NSS genérica.
    Nssdb,
    /// Almacén correspondiente a un fichero PKCS#12 instalado.
    Installed,
}

/// Almacén de certificados PKCS#11 o NSS con sus parámetros de apertura.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Store {
    module: PathBuf,
    init_args: Option<String>,
}

impl Store {
    /// Construye un almacén PKCS#11 estándar sin parámetros adicionales.
    pub fn module(module: impl Into<PathBuf>) -> Self {
        Self {
            module: module.into(),
            init_args: None,
        }
    }

    /// Construye un almacén con parámetros de inicialización específicos.
    pub fn with_init_args(module: impl Into<PathBuf>, init_args: Option<String>) -> Self {
        Self {
            module: module.into(),
            init_args,
        }
    }

    /// Construye un almacén NSS en modo de solo lectura para un perfil.
    pub fn nss(softoken: impl Into<PathBuf>, profile: &Path) -> Self {
        Self {
            module: softoken.into(),
            init_args: Some(format!(
                "configdir='sql:{}' certPrefix='' keyPrefix='' secmod='secmod.db' flags=readOnly",
                profile.display()
            )),
        }
    }

    /// Ruta del módulo PKCS#11 que lo sirve.
    pub fn path(&self) -> &Path {
        &self.module
    }

    /// Parámetros de inicialización requeridos por el módulo.
    pub fn init_args(&self) -> Option<&str> {
        self.init_args.as_deref()
    }

    /// Clasifica el tipo de almacén considerando el directorio de instalación (ADR-0011).
    pub fn class_under(&self, installed_dir: &Path) -> StoreClass {
        if self.installed_directory_under(installed_dir).is_some() {
            StoreClass::Installed
        } else {
            self.class()
        }
    }

    /// Clasifica el tipo de almacén según sus parámetros.
    pub fn class(&self) -> StoreClass {
        let Some(profile) = self.profile() else {
            return StoreClass::Card;
        };
        if profile.contains("/.mozilla/firefox/") || profile.contains("/mozilla/firefox/") {
            StoreClass::Firefox
        } else if profile.ends_with("/.pki/nssdb") || profile.ends_with("/pki/nssdb") {
            StoreClass::Chrome
        } else {
            StoreClass::Nssdb
        }
    }

    /// Directorio del perfil NSS si está configurado.
    fn profile(&self) -> Option<&str> {
        let args = self.init_args.as_deref()?;
        let after = args.split_once("configdir='")?.1;
        let inside = after.split_once('\'')?.0;
        Some(inside.strip_prefix("sql:").unwrap_or(inside))
    }

    /// Directorio del almacén si corresponde a un PKCS#12 instalado (ADR-0011).
    pub fn installed_directory_under(&self, installed_dir: &Path) -> Option<PathBuf> {
        let directory = PathBuf::from(self.profile()?);
        (directory.parent() == Some(installed_dir) && directory.join("cert9.db").is_file())
            .then_some(directory)
    }
}

impl From<&Path> for Store {
    fn from(module: &Path) -> Self {
        Self::module(module)
    }
}

impl From<PathBuf> for Store {
    fn from(module: PathBuf) -> Self {
        Self::module(module)
    }
}

impl From<&PathBuf> for Store {
    fn from(module: &PathBuf) -> Self {
        Self::module(module.clone())
    }
}

impl From<&str> for Store {
    fn from(module: &str) -> Self {
        Self::module(module)
    }
}

impl From<&Store> for Store {
    fn from(store: &Store) -> Self {
        store.clone()
    }
}

/// Descubre los almacenes disponibles en el entorno actual.
pub fn from_environment() -> Vec<Store> {
    if let Some(module) = std::env::var_os(crate::PKCS11_MODULE_VARIABLE) {
        return vec![Store::module(module)];
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut stores: Vec<Store> = present_among(CANDIDATE_MODULES, |path| path.is_file())
        .into_iter()
        .map(Store::module)
        .collect();

    if let (Some(home), Some(softoken)) = (home, softoken()) {
        stores.extend(
            nss_profiles(&home)
                .into_iter()
                .map(|profile| Store::nss(&softoken, &profile)),
        );
    }

    stores
}

/// Localiza la biblioteca softoken de NSS en el sistema.
pub fn softoken() -> Option<PathBuf> {
    present_among(CANDIDATE_SOFTOKENS, |path| path.is_file())
        .into_iter()
        .next()
}

/// Obtiene los almacenes correspondientes a ficheros PKCS#12 instalados.
pub fn installed_stores(softoken: &Path, directory: &Path) -> Vec<Store> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut installed: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("cert9.db").is_file())
        .collect();
    installed.sort();
    installed
        .iter()
        .map(|profile| Store::nss(softoken, profile))
        .collect()
}

/// Pares de directorios de configuración y datos de Firefox en el sistema.
fn firefox_layouts(home: &Path) -> [(PathBuf, PathBuf); 2] {
    [
        (home.join(".mozilla/firefox"), home.join(".mozilla/firefox")),
        (
            home.join(".config/mozilla/firefox"),
            home.join(".local/share/mozilla/firefox"),
        ),
    ]
}

/// Descubre las rutas de perfiles NSS existentes bajo el directorio personal.
pub fn nss_profiles(home: &Path) -> Vec<PathBuf> {
    let mut profiles: Vec<PathBuf> = Vec::new();
    for (config, data) in firefox_layouts(home) {
        profiles.extend(
            profiles_declared_in(&config.join("profiles.ini"))
                .into_iter()
                .map(|relative_or_absolute| resolve_under(&data, &relative_or_absolute)),
        );
    }
    profiles.push(home.join(".pki/nssdb"));
    profiles.push(home.join(".local/share/pki/nssdb"));

    let mut found: Vec<PathBuf> = Vec::new();
    for profile in profiles {
        if !profile.join("cert9.db").is_file() {
            continue;
        }
        let resolved = profile.canonicalize().unwrap_or_else(|_| profile.clone());
        if !found
            .iter()
            .any(|already| already.canonicalize().unwrap_or_else(|_| already.clone()) == resolved)
        {
            found.push(profile);
        }
    }

    found
}

/// Rutas de perfiles declaradas en un fichero profiles.ini.
fn profiles_declared_in(ini: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(ini) else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    let mut inside_a_profile = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            inside_a_profile = section.starts_with("Profile");
            continue;
        }
        if !inside_a_profile {
            continue;
        }
        if let Some(value) = line.strip_prefix("Path=") {
            let value = value.trim();
            if !value.is_empty() {
                paths.push(value.to_owned());
            }
        }
    }

    paths
}

/// Resuelve una ruta de perfil relativa o absoluta.
fn resolve_under(firefox: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        firefox.join(path)
    }
}

/// Filtra y deduplica rutas existentes entre las candidatas indicadas.
pub fn present_among(candidates: &[&str], present: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut stores: Vec<PathBuf> = Vec::new();

    for candidate in candidates {
        let path = Path::new(candidate);
        if !present(path) {
            continue;
        }
        let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let already = stores
            .iter()
            .any(|store| store.canonicalize().unwrap_or_else(|_| store.clone()) == resolved);
        if !already {
            stores.push(path.to_path_buf());
        }
    }

    stores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_candidates_that_are_there() {
        let stores = present_among(&["/hay/uno.so", "/no/hay.so", "/hay/otro.so"], |path| {
            path.starts_with("/hay")
        });

        assert_eq!(
            stores,
            vec![PathBuf::from("/hay/uno.so"), PathBuf::from("/hay/otro.so")]
        );
    }

    #[test]
    fn a_store_under_the_installed_directory_is_its_own_class() {
        let home = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
        let installed = home.path().join("certificates");
        let mine = installed.join("2a01");
        std::fs::create_dir_all(&mine).expect("deberia poder crearse el almacen");
        std::fs::write(mine.join("cert9.db"), b"").expect("deberia poder escribirse cert9.db");
        let elsewhere = home.path().join(".pki/nssdb");
        std::fs::create_dir_all(&elsewhere).expect("deberia poder crearse el perfil");

        assert_eq!(
            Store::nss("/usr/lib/libsoftokn3.so", &mine).class_under(&installed),
            StoreClass::Installed
        );
        assert_eq!(
            Store::nss("/usr/lib/libsoftokn3.so", &elsewhere).class_under(&installed),
            StoreClass::Chrome
        );
    }

    #[test]
    fn has_no_stores_when_no_candidate_is_installed() {
        assert!(present_among(CANDIDATE_MODULES, |_| false).is_empty());
    }

    #[test]
    fn lists_the_same_module_once_even_under_two_names() {
        let directory = tempfile::tempdir().expect("deberia poder crearse un directorio temporal");
        let module = directory.path().join("modulo.so");
        std::fs::write(&module, b"").expect("deberia poder escribirse el modulo");
        let link = directory.path().join("enlace.so");
        std::os::unix::fs::symlink(&module, &link).expect("deberia poder enlazarse");

        let candidates = [
            module.to_str().expect("ruta valida"),
            link.to_str().expect("ruta valida"),
        ];
        let stores = present_among(&candidates, |path| path.is_file());

        assert_eq!(stores, vec![module]);
    }

    #[test]
    fn a_plain_module_has_nothing_to_configure() {
        assert_eq!(Store::module("/usr/lib/x.so").init_args(), None);
    }

    #[test]
    fn a_plain_module_is_a_card() {
        assert_eq!(Store::module("/usr/lib/x.so").class(), StoreClass::Card);
    }

    #[test]
    fn an_nss_store_is_classified_by_whose_profile_it_opens() {
        let firefox = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.mozilla/firefox/aaaaaaaa.default-release"),
        );
        let chrome = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/ada/.pki/nssdb"));

        assert_eq!(firefox.class(), StoreClass::Firefox);
        assert_eq!(chrome.class(), StoreClass::Chrome);
    }

    #[test]
    fn an_nss_store_is_classified_the_same_under_the_xdg_paths() {
        let firefox = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.local/share/mozilla/firefox/cccccccc.default-release"),
        );
        let chrome = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/casa/ada/.local/share/pki/nssdb"),
        );

        assert_eq!(firefox.class(), StoreClass::Firefox);
        assert_eq!(chrome.class(), StoreClass::Chrome);
    }

    #[test]
    fn an_nss_store_somewhere_else_claims_no_owner() {
        let store = Store::nss(
            "/usr/lib/libsoftokn3.so",
            Path::new("/tmp/perfil-de-pruebas"),
        );

        assert_eq!(store.class(), StoreClass::Nssdb);
    }

    #[test]
    fn an_nss_store_opens_the_profile_read_only_and_in_sql_format() {
        let store = Store::nss("/usr/lib/libsoftokn3.so", Path::new("/casa/perfil"));
        let args = store.init_args().expect("un almacen NSS lleva init args");

        assert!(args.contains("configdir='sql:/casa/perfil'"), "{args}");
        assert!(args.contains("flags=readOnly"), "{args}");
    }

    fn a_home_with(profiles: &[(&str, bool)], ini: Option<&str>) -> tempfile::TempDir {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let firefox = home.path().join(".mozilla/firefox");
        std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
        for (name, with_database) in profiles {
            let directory = home.path().join(name);
            std::fs::create_dir_all(&directory).expect("deberia poder crearse el perfil");
            if *with_database {
                std::fs::write(directory.join("cert9.db"), b"").expect("deberia poder escribirse");
            }
        }
        if let Some(ini) = ini {
            std::fs::write(firefox.join("profiles.ini"), ini).expect("deberia poder escribirse");
        }
        home
    }

    #[test]
    fn reads_every_firefox_profile_declared_in_profiles_ini() {
        let home = a_home_with(
            &[
                (".mozilla/firefox/aaaaaaaa.default-release", true),
                (".mozilla/firefox/bbbbbbbb.trabajo", true),
            ],
            Some(
                "[Install4F96D1932A9F858E]\n\
                 Default=aaaaaaaa.default-release\n\
                 \n\
                 [Profile0]\n\
                 Name=default-release\n\
                 IsRelative=1\n\
                 Path=aaaaaaaa.default-release\n\
                 \n\
                 [Profile1]\n\
                 Name=trabajo\n\
                 IsRelative=1\n\
                 Path=bbbbbbbb.trabajo\n",
            ),
        );

        assert_eq!(
            nss_profiles(home.path()),
            vec![
                home.path()
                    .join(".mozilla/firefox/aaaaaaaa.default-release"),
                home.path().join(".mozilla/firefox/bbbbbbbb.trabajo"),
            ]
        );
    }

    #[test]
    fn skips_a_declared_profile_without_a_certificate_database() {
        let home = a_home_with(
            &[
                (".mozilla/firefox/aaaaaaaa.vacio", false),
                (".mozilla/firefox/bbbbbbbb.lleno", true),
            ],
            Some("[Profile0]\nPath=aaaaaaaa.vacio\n\n[Profile1]\nPath=bbbbbbbb.lleno\n"),
        );

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".mozilla/firefox/bbbbbbbb.lleno")]
        );
    }

    #[test]
    fn reads_the_shared_nssdb_too() {
        let home = a_home_with(&[(".pki/nssdb", true)], None);

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".pki/nssdb")]
        );
    }

    #[test]
    fn has_no_profiles_when_firefox_is_not_installed() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");

        assert!(nss_profiles(home.path()).is_empty());
    }

    #[test]
    fn reads_a_firefox_profile_from_the_paired_xdg_config_and_data_dirs() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let config = home.path().join(".config/mozilla/firefox");
        let data = home.path().join(".local/share/mozilla/firefox");
        let profile = data.join("cccccccc.default-release");
        std::fs::create_dir_all(&config).expect("deberia poder crearse .config/mozilla/firefox");
        std::fs::create_dir_all(&profile).expect("deberia poder crearse el perfil");
        std::fs::write(profile.join("cert9.db"), b"").expect("deberia poder escribirse");
        std::fs::write(
            config.join("profiles.ini"),
            "[Profile0]\nPath=cccccccc.default-release\n",
        )
        .expect("deberia poder escribirse");

        assert_eq!(nss_profiles(home.path()), vec![profile]);
    }

    #[test]
    fn reads_the_xdg_shared_nssdb_too() {
        let home = a_home_with(&[(".local/share/pki/nssdb", true)], None);

        assert_eq!(
            nss_profiles(home.path()),
            vec![home.path().join(".local/share/pki/nssdb")]
        );
    }

    #[test]
    fn resolves_an_absolute_profile_path_as_it_comes() {
        let home = tempfile::tempdir().expect("deberia poder crearse un HOME de mentira");
        let firefox = home.path().join(".mozilla/firefox");
        let elsewhere = home.path().join("otro-sitio");
        std::fs::create_dir_all(&firefox).expect("deberia poder crearse .mozilla/firefox");
        std::fs::create_dir_all(&elsewhere).expect("deberia poder crearse el otro sitio");
        std::fs::write(elsewhere.join("cert9.db"), b"").expect("deberia poder escribirse");
        std::fs::write(
            firefox.join("profiles.ini"),
            format!("[Profile0]\nIsRelative=0\nPath={}\n", elsewhere.display()),
        )
        .expect("deberia poder escribirse");

        assert_eq!(nss_profiles(home.path()), vec![elsewhere]);
    }
}
