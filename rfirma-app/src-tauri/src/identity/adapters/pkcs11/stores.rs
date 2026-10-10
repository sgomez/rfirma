//! Dónde se buscan los certificados —almacenes PKCS#11, perfiles NSS y `.p12` instalados— y qué módulo descubierto es la biblioteca que nombra la sede (ADR-0022).

use std::path::{Path, PathBuf};

use super::p11kit;
use crate::desktop::adapters::channel::Channel;
use crate::identity::domain::store::{Store, StoreClass};

/// Rutas candidatas fijas para módulos PKCS#11 estándar.
pub const CANDIDATE_MODULES: &[&str] = &["/usr/lib/softhsm/libsofthsm2.so"];

/// Rutas candidatas fijas para bibliotecas softoken de NSS.
pub const CANDIDATE_SOFTOKENS: &[&str] = &[
    "/usr/lib64/libsoftokn3.so",
    "/usr/lib64/nss/libsoftokn3.so",
    "/usr/lib/libsoftokn3.so",
    "/usr/lib/nss/libsoftokn3.so",
];

/// Subdirectorios multiarch bajo el directorio de librerías indicado.
pub fn multiarch_subdirectories(usr_lib: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(usr_lib) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.contains("-linux-"))
                    .unwrap_or(false)
        })
        .collect();
    dirs.sort();
    dirs
}

/// Rutas candidatas para bibliotecas softoken de NSS bajo el directorio de librerías indicado.
pub fn candidate_softokens_under(usr_lib: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in multiarch_subdirectories(usr_lib) {
        candidates.push(dir.join("libsoftokn3.so"));
        candidates.push(dir.join("nss/libsoftokn3.so"));
    }
    candidates.extend(CANDIDATE_SOFTOKENS.iter().map(PathBuf::from));
    candidates
}

/// Rutas candidatas para bibliotecas softoken de NSS en el sistema.
pub fn candidate_softokens() -> Vec<PathBuf> {
    candidate_softokens_under(Path::new("/usr/lib"))
}

/// Rutas candidatas para módulos PKCS#11 estándar bajo el directorio de librerías indicado.
pub fn candidate_modules_under(usr_lib: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for dir in multiarch_subdirectories(usr_lib) {
        candidates.push(dir.join("softhsm/libsofthsm2.so"));
    }
    candidates.extend(CANDIDATE_MODULES.iter().map(PathBuf::from));
    candidates
}

/// Los módulos PKCS#11 instalados bajo `usr`: los candidatos fijos y los registrados en p11-kit.
pub fn discovered_modules(
    usr: &Path,
    app: Option<&Path>,
    p11kit_directories: &[PathBuf],
) -> Vec<PathBuf> {
    let registered = p11kit::registered_modules(p11kit_directories, usr, app);
    present_among(
        candidate_modules_under(&usr.join("lib"))
            .into_iter()
            .chain(registered),
        |path| path.is_file(),
    )
}

/// Un módulo PKCS#11 que el descubrimiento ha visto, usado o descartado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredModule {
    /// Nombre del `.module`, o de la biblioteca si es un candidato fijo.
    pub name: String,
    /// La biblioteca; la del descartado, tal como el `.module` la nombra.
    pub library: Option<PathBuf>,
    /// El `.module` que lo da de alta; ninguno en los candidatos fijos.
    pub registration: Option<PathBuf>,
    /// Por qué no se usa; `None` si se usa.
    pub discard: Option<p11kit::DiscardReason>,
}

/// Lo que `discovered_modules` usa y lo que descarta con su motivo, sin cargar nada.
pub fn module_discovery(
    usr: &Path,
    app: Option<&Path>,
    p11kit_directories: &[PathBuf],
) -> Vec<DiscoveredModule> {
    let registrations = p11kit::registrations(p11kit_directories, usr, app);
    let registration_of = |module: &Path| {
        registrations.iter().find(|registration| {
            registration
                .outcome
                .as_ref()
                .is_ok_and(|library| same_file(library, module))
        })
    };
    let mut report: Vec<DiscoveredModule> = discovered_modules(usr, app, p11kit_directories)
        .into_iter()
        .map(|module| match registration_of(&module) {
            Some(registration) => DiscoveredModule {
                name: registration.name.clone(),
                library: Some(module),
                registration: Some(registration.file.clone()),
                discard: None,
            },
            None => DiscoveredModule {
                name: module
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                library: Some(module),
                registration: None,
                discard: None,
            },
        })
        .collect();
    report.extend(registrations.iter().filter_map(|registration| {
        Some(DiscoveredModule {
            name: registration.name.clone(),
            library: registration.library.as_ref().map(PathBuf::from),
            registration: Some(registration.file.clone()),
            discard: Some(*registration.outcome.as_ref().err()?),
        })
    }));
    report
}

fn same_file(first: &Path, second: &Path) -> bool {
    first.canonicalize().ok() == second.canonicalize().ok()
}

/// El módulo PKCS#11 descubierto que es, canonizada, la biblioteca que nombra la sede.
pub fn discovered_module_named(stores: &[Store], library: &str) -> Option<PathBuf> {
    let named = Path::new(library).canonicalize().ok()?;
    stores
        .iter()
        .filter(|store| store.class() == StoreClass::Card)
        .map(Store::path)
        .find(|module| module.canonicalize().is_ok_and(|module| module == named))
        .map(Path::to_path_buf)
}

/// Descubre los almacenes disponibles en el entorno actual.
pub fn from_environment() -> Vec<Store> {
    if let Some(module) = std::env::var_os(crate::PKCS11_MODULE_VARIABLE) {
        return vec![Store::module(module)];
    }

    let home = std::env::var_os("HOME").map(PathBuf::from);
    let channel = Channel::detected();
    let p11kit_directories = environment_p11kit_directories(home.as_deref(), channel);
    let mut stores: Vec<Store> = discovered_modules(
        Path::new("/usr"),
        p11kit::app_root(channel),
        &p11kit_directories,
    )
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

/// Lo que el descubrimiento de este proceso usa y descarta, salvo que `RFIRMA_PKCS11_MODULE` lo anule.
pub fn modules_from_environment() -> Vec<DiscoveredModule> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let channel = Channel::detected();
    module_discovery(
        Path::new("/usr"),
        p11kit::app_root(channel),
        &environment_p11kit_directories(home.as_deref(), channel),
    )
}

fn environment_p11kit_directories(home: Option<&Path>, channel: Channel) -> Vec<PathBuf> {
    home.map(|home| p11kit::configuration_directories(home, channel))
        .unwrap_or_default()
}

/// Localiza la biblioteca softoken de NSS en el sistema.
pub fn softoken() -> Option<PathBuf> {
    softoken_under(Path::new("/usr/lib"))
}

/// Localiza la biblioteca softoken de NSS bajo el directorio de librerías indicado.
pub fn softoken_under(usr_lib: &Path) -> Option<PathBuf> {
    present_among(candidate_softokens_under(usr_lib), |path| path.is_file())
        .into_iter()
        .next()
}

/// El Almacén de rFirma bajo `directory`, si ya se ha instalado algún certificado (ADR-0034).
pub fn installed_stores(softoken: &Path, directory: &Path) -> Vec<Store> {
    if directory.join("cert9.db").is_file() {
        vec![Store::installed_nss(softoken, directory)]
    } else {
        Vec::new()
    }
}

/// Pares de directorios de configuración y datos de Firefox en el sistema.
fn firefox_layouts(home: &Path) -> [(PathBuf, PathBuf); 7] {
    let flatpak = home.join(".var/app/org.mozilla.firefox");
    [
        (home.join(".mozilla/firefox"), home.join(".mozilla/firefox")),
        (
            home.join(".config/mozilla/firefox"),
            home.join(".local/share/mozilla/firefox"),
        ),
        (
            flatpak.join(".mozilla/firefox"),
            flatpak.join(".mozilla/firefox"),
        ),
        (
            flatpak.join("config/mozilla/firefox"),
            flatpak.join("data/mozilla/firefox"),
        ),
        (
            home.join("snap/firefox/common/.mozilla/firefox"),
            home.join("snap/firefox/common/.mozilla/firefox"),
        ),
        (home.join(".librewolf"), home.join(".librewolf")),
        (
            home.join(".config/librewolf/librewolf"),
            home.join(".config/librewolf/librewolf"),
        ),
    ]
}

/// Descubre las rutas de perfiles NSS existentes bajo el directorio personal.
pub fn nss_profiles(home: &Path) -> Vec<PathBuf> {
    let mut profiles: Vec<PathBuf> = Vec::new();
    for (config, data) in firefox_layouts(home) {
        for relative_or_absolute in profiles_declared_in(&config.join("profiles.ini")) {
            profiles.push(resolve_under(&data, &relative_or_absolute));
            profiles.push(resolve_under(&config, &relative_or_absolute));
        }
    }
    profiles.push(home.join(".pki/nssdb"));
    profiles.push(home.join(".local/share/pki/nssdb"));
    profiles.push(home.join("snap/chromium/current/.local/share/pki/nssdb"));
    profiles.push(home.join("snap/chromium/current/.pki/nssdb"));

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
pub fn present_among<P: AsRef<Path>>(
    candidates: impl IntoIterator<Item = P>,
    present: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut stores: Vec<PathBuf> = Vec::new();

    for candidate in candidates {
        let path = candidate.as_ref();
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
