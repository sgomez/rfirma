//! Los módulos PKCS#11 que la instalación registra en p11-kit; no carga ninguno.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::stores::multiarch_subdirectories;
use crate::desktop::domain::channel::Channel;

/// El nombre de programa con el que rFirma se busca en `enable-in` y `disable-in`.
pub const PROGRAM_NAME: &str = "rfirma";

const TRUST_ONLY_LIBRARIES: &[&str] = &["p11-kit-trust.so", "p11-kit-client.so"];

/// La raíz `/app` donde el paquete registra sus módulos; solo existe en el flatpak (ADR-0049).
pub fn app_root(channel: Channel) -> Option<&'static Path> {
    (channel == Channel::Flatpak).then(|| Path::new("/app"))
}

/// Los directorios de ficheros `.module`, de menos a más prioridad.
pub fn configuration_directories(home: &Path, channel: Channel) -> Vec<PathBuf> {
    let mut directories = vec![PathBuf::from("/usr/share/p11-kit/modules")];
    directories.extend(app_root(channel).map(|app| app.join("share/p11-kit/modules")));
    directories.push(PathBuf::from("/etc/pkcs11/modules"));
    directories.push(home.join(".config/pkcs11/modules"));
    directories
}

/// Por qué rFirma no usa un módulo que p11-kit tiene dado de alta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscardReason {
    /// Su `disable-in` nombra a rFirma.
    DisabledInRfirma,
    /// Su `enable-in` no nombra a rFirma.
    EnabledOnlyElsewhere,
    /// Es un almacén de confianza, no de claves.
    TrustPolicy,
    /// Su biblioteca no está instalada.
    MissingModule,
}

/// Un fichero `.module`, con lo que se hizo con él.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
    /// Nombre del fichero sin la extensión.
    pub name: String,
    /// El fichero `.module` que gana.
    pub file: PathBuf,
    /// La biblioteca tal como el fichero la nombra.
    pub library: Option<String>,
    /// La biblioteca instalada que se usa, o el motivo de descarte.
    pub outcome: Result<PathBuf, DiscardReason>,
}

struct Verdict {
    library: Option<String>,
    usable: Result<String, DiscardReason>,
}

fn verdict(config: &str) -> Option<Verdict> {
    let value = |key: &str| {
        config.lines().find_map(|line| {
            let (found, value) = line.split_once(':')?;
            (found.trim() == key).then(|| value.trim().to_owned())
        })
    };
    let names_rfirma = |programs: String| {
        programs
            .split(|c: char| c == ',' || c.is_whitespace())
            .any(|program| program == PROGRAM_NAME)
    };
    let library = value("module").filter(|module| !module.is_empty());
    let discarded = |reason| {
        Some(Verdict {
            library: library.clone(),
            usable: Err(reason),
        })
    };

    if value("trust-policy").is_some_and(|policy| policy.eq_ignore_ascii_case("yes")) {
        return discarded(DiscardReason::TrustPolicy);
    }
    if value("enable-in").is_some_and(|programs| !names_rfirma(programs)) {
        return discarded(DiscardReason::EnabledOnlyElsewhere);
    }
    if value("disable-in").is_some_and(names_rfirma) {
        return discarded(DiscardReason::DisabledInRfirma);
    }
    match &library {
        Some(module) if is_trust_module(module) => discarded(DiscardReason::TrustPolicy),
        Some(module) => Some(Verdict {
            library: library.clone(),
            usable: Ok(module.clone()),
        }),
        None => None,
    }
}

/// La biblioteca que un fichero `.module` registra para rFirma, si la registra.
pub fn module_for_rfirma(config: &str) -> Option<String> {
    verdict(config)?.usable.ok()
}

fn is_trust_module(module: &str) -> bool {
    Path::new(module)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| TRUST_ONLY_LIBRARIES.contains(&name))
}

/// Las bibliotecas registradas en esos directorios, resueltas bajo `usr` y `app` e instaladas.
pub fn registered_modules(directories: &[PathBuf], usr: &Path, app: Option<&Path>) -> Vec<PathBuf> {
    registrations(directories, usr, app)
        .into_iter()
        .filter_map(|registration| registration.outcome.ok())
        .collect()
}

/// Cada `.module` que registra o descarta una biblioteca, con su motivo.
pub fn registrations(directories: &[PathBuf], usr: &Path, app: Option<&Path>) -> Vec<Registration> {
    module_files(directories)
        .into_iter()
        .filter_map(|(file_name, file)| {
            let verdict = verdict(&std::fs::read_to_string(&file).ok()?)?;
            let outcome = verdict.usable.and_then(|module| {
                installed(&module, usr, app).ok_or(DiscardReason::MissingModule)
            });
            Some(Registration {
                name: file_name.trim_end_matches(".module").to_owned(),
                file,
                library: verdict.library,
                outcome,
            })
        })
        .collect()
}

/// Los `.module` por nombre de fichero: el de un directorio posterior tapa al anterior.
fn module_files(directories: &[PathBuf]) -> BTreeMap<String, PathBuf> {
    let mut files = BTreeMap::new();
    for directory in directories {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.ends_with(".module") && path.is_file() {
                files.insert(name.to_owned(), path);
            }
        }
    }
    files
}

/// La absoluta, si está; la relativa, en el primer directorio de módulos que la tenga.
fn installed(module: &str, usr: &Path, app: Option<&Path>) -> Option<PathBuf> {
    let module = Path::new(module);
    if module.is_absolute() {
        return module.is_file().then(|| module.to_path_buf());
    }
    module_directories(usr)
        .into_iter()
        .chain(app.map(|app| app.join("lib/pkcs11")))
        .map(|directory| directory.join(module))
        .find(|path| path.is_file())
}

/// Donde p11-kit instala sus módulos en Debian, Fedora y Arch.
fn module_directories(usr: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<PathBuf> = multiarch_subdirectories(&usr.join("lib"))
        .into_iter()
        .map(|multiarch| multiarch.join("pkcs11"))
        .collect();
    directories.push(usr.join("lib64/pkcs11"));
    directories.push(usr.join("lib/pkcs11"));
    directories
}

#[cfg(test)]
mod tests;
