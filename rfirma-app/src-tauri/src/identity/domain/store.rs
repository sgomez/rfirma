//! Un almacén de certificados: la ruta de su módulo y cómo se abre, sin abrirlo.

use std::path::{Path, PathBuf};

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
