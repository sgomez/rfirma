//! Registro del manejador predeterminado para un esquema en mimeapps.list (ADR-0015).

use super::{content_type_for, Channel};
use crate::desktop::error::{DesktopError, Situation};
use crate::paths::{xdg_config_home, HomeUnknown};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Nombre del fichero de asociaciones predeterminadas del usuario.
const MIMEAPPS_LIST: &str = "mimeapps.list";

/// Sección del fichero donde se declaran las aplicaciones predeterminadas.
const DEFAULT_APPLICATIONS: &str = "[Default Applications]";

/// Circunstancias externas que pueden condicionar la preferencia registrada.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceOverride {
    /// Firefox mantiene su propia tabla interna de asociaciones en sus preferencias.
    FirefoxKeepsItsOwn,
}

/// Lista de circunstancias externas reconocidas.
const WHAT_CAN_OVERRIDE_THE_CHOICE: [ChoiceOverride; 1] = [ChoiceOverride::FirefoxKeepsItsOwn];

/// Resultado del registro de un manejador predeterminado junto con posibles advertencias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceWritten {
    list: PathBuf,
    overridden_by: &'static [ChoiceOverride],
}

impl ChoiceWritten {
    /// Fichero mimeapps.list donde se registró la preferencia.
    pub fn list(&self) -> &Path {
        &self.list
    }

    /// Circunstancias externas que pueden condicionar la preferencia.
    pub fn overridden_by(&self) -> &'static [ChoiceOverride] {
        self.overridden_by
    }
}

/// Obtiene la ruta de mimeapps.list a partir de las variables de entorno actuales.
pub fn mimeapps_list_from_environment() -> Result<PathBuf, HomeUnknown> {
    mimeapps_list(&|name| std::env::var_os(name))
}

/// Obtiene la ruta de mimeapps.list resolviendo el entorno con una función provista.
pub fn mimeapps_list(
    environment: &dyn Fn(&str) -> Option<OsString>,
) -> Result<PathBuf, HomeUnknown> {
    Ok(xdg_config_home(environment)?.join(MIMEAPPS_LIST))
}

/// Registra el manejador predeterminado para un esquema en el fichero mimeapps.list.
pub fn choose_handler_for_scheme(
    channel: Channel,
    list: &Path,
    scheme: &str,
    handler: &str,
) -> Result<ChoiceWritten, DesktopError> {
    if channel == Channel::Flatpak {
        return Err(DesktopError::new(
            Situation::NotAvailableInsideTheSandbox,
            format!("{} no es el del anfitrión", list.display()),
        ));
    }
    let current = match fs::read_to_string(list) {
        Ok(current) => current,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(DesktopError::new(
                Situation::TheListIsNotReadable,
                format!("{}: {error}", list.display()),
            ))
        }
    };
    let updated = with_explicit_default(&current, &content_type_for(scheme), handler);
    write_atomically(list, updated.as_bytes()).map_err(|error| {
        DesktopError::new(
            Situation::TheListIsNotWritable,
            format!("{}: {error}", list.display()),
        )
    })?;
    Ok(ChoiceWritten {
        list: list.to_path_buf(),
        overridden_by: &WHAT_CAN_OVERRIDE_THE_CHOICE,
    })
}

/// Consulta el manejador predeterminado configurado actualmente para un esquema.
pub fn current_default_for_scheme(channel: Channel, list: &Path, scheme: &str) -> Option<String> {
    if channel == Channel::Flatpak {
        return None;
    }
    let content = fs::read_to_string(list).ok()?;
    default_in(&content, &content_type_for(scheme))
}

/// Busca el valor predeterminado para un tipo de contenido en el texto del fichero.
fn default_in(content: &str, content_type: &str) -> Option<String> {
    let mut inside = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == DEFAULT_APPLICATIONS;
            continue;
        }
        if !inside || key_of(line) != Some(content_type) {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        return value
            .split(';')
            .map(str::trim)
            .find(|entry| !entry.is_empty())
            .map(str::to_owned);
    }
    None
}

/// Inserta o actualiza la clave del esquema dentro de [Default Applications].
fn with_explicit_default(content: &str, content_type: &str, handler: &str) -> String {
    let entry = format!("{content_type}={handler};");
    let mut lines: Vec<String> = Vec::new();
    let mut inside_the_group = false;
    let mut written = false;

    for line in content.lines() {
        if line.trim_start().starts_with('[') {
            if inside_the_group && !written {
                insert_before_the_blank_tail(&mut lines, &entry);
                written = true;
            }
            inside_the_group = line.trim() == DEFAULT_APPLICATIONS;
            lines.push(line.to_owned());
            continue;
        }
        if inside_the_group && key_of(line) == Some(content_type) {
            if !written {
                lines.push(entry.clone());
                written = true;
            }
            continue;
        }
        lines.push(line.to_owned());
    }

    if inside_the_group && !written {
        insert_before_the_blank_tail(&mut lines, &entry);
        written = true;
    }
    if !written {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(DEFAULT_APPLICATIONS.to_owned());
        lines.push(entry);
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

/// Extrae la clave de una línea clave=valor si no está comentada.
fn key_of(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(key, _)| key.trim())
}

/// Inserta una línea antes de las líneas vacías finales del bloque.
fn insert_before_the_blank_tail(lines: &mut Vec<String>, entry: &str) {
    let blanks = lines
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count();
    lines.insert(lines.len() - blanks, entry.to_owned());
}

/// Escribe el contenido atómicamente mediante un fichero temporal.
fn write_atomically(path: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".rfirma.tmp");
    let temporary = PathBuf::from(temporary);
    let written = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(content)?;
        file.sync_all()
    })();
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

#[cfg(test)]
mod tests;
