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
mod tests;
