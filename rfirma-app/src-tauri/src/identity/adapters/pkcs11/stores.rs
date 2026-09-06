//! Colección y descubrimiento de almacenes PKCS#11 y perfiles NSS.

use std::path::{Path, PathBuf};

pub use crate::identity::domain::store::{Store, StoreClass};

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
