//! Los módulos PKCS#11 que la instalación registra en p11-kit; no carga ninguno.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::stores::multiarch_subdirectories;

/// El nombre de programa con el que rFirma se busca en `enable-in` y `disable-in`.
pub const PROGRAM_NAME: &str = "rfirma";

/// Los directorios de ficheros `.module`, de menos a más prioridad.
pub fn configuration_directories(home: &Path) -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/p11-kit/modules"),
        PathBuf::from("/etc/pkcs11/modules"),
        home.join(".config/pkcs11/modules"),
    ]
}

/// La biblioteca que un fichero `.module` registra para rFirma, si la registra.
pub fn module_for_rfirma(config: &str) -> Option<String> {
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

    if value("trust-policy").is_some_and(|policy| policy.eq_ignore_ascii_case("yes")) {
        return None;
    }
    if value("enable-in").is_some_and(|programs| !names_rfirma(programs)) {
        return None;
    }
    if value("disable-in").is_some_and(names_rfirma) {
        return None;
    }
    value("module").filter(|module| !module.is_empty())
}

/// Las bibliotecas registradas en esos directorios, resueltas bajo `usr` e instaladas.
pub fn registered_modules(directories: &[PathBuf], usr: &Path) -> Vec<PathBuf> {
    module_files(directories)
        .values()
        .filter_map(|file| std::fs::read_to_string(file).ok())
        .filter_map(|config| module_for_rfirma(&config))
        .filter_map(|module| installed(&module, usr))
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
fn installed(module: &str, usr: &Path) -> Option<PathBuf> {
    let module = Path::new(module);
    if module.is_absolute() {
        return module.is_file().then(|| module.to_path_buf());
    }
    module_directories(usr)
        .into_iter()
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
