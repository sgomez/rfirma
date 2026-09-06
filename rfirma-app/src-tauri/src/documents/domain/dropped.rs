//! Clasificación de ficheros soltados en la ventana o recibidos por línea de órdenes (ADR-0011).

use std::path::{Path, PathBuf};

/// Resultado de clasificar los ficheros soltados o recibidos.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// El primer PDF legible y el resto de documentos que entran en recientes.
    Opened {
        path: PathBuf,
        also_entering: Vec<PathBuf>,
        discarded: usize,
    },
    /// Ninguno de los ficheros soltados o recorridos es un PDF.
    NotAPdf { discarded: usize },
    /// El primer PDF no se ha podido leer.
    Unreadable { detail: String, discarded: usize },
    /// No se ha proporcionado ningún fichero.
    Nothing,
}

const PDF: &str = "pdf";

/// Clasifica los ficheros soltados y selecciona el primer PDF legible.
pub fn first_pdf(paths: &[PathBuf]) -> Dropped {
    if paths.is_empty() {
        return Dropped::Nothing;
    }
    let candidates = expand_folders(paths);
    let discarded = candidates.iter().filter(|path| !is_pdf(path)).count();
    let mut pdfs = candidates.iter().filter(|path| is_pdf(path));
    let Some(first) = pdfs.next() else {
        return Dropped::NotAPdf { discarded };
    };
    match std::fs::File::open(first) {
        Ok(_) => Dropped::Opened {
            path: first.clone(),
            also_entering: pdfs.cloned().collect(),
            discarded,
        },
        Err(error) => Dropped::Unreadable {
            detail: error.to_string(),
            discarded: candidates.len() - 1,
        },
    }
}

/// Extrae y clasifica el PDF recibido por línea de órdenes si existe.
pub fn invoked_pdf(command_line: &[String], from: &Path) -> Dropped {
    let paths: Vec<PathBuf> = command_line
        .iter()
        .skip(1)
        .filter(|argument| !argument.starts_with('-'))
        .map(|argument| from.join(argument))
        .collect();
    first_pdf(&paths)
}

fn expand_folders(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut expanded = Vec::with_capacity(paths.len());
    for path in paths {
        if path.is_dir() {
            expanded.extend(files_within(path));
        } else {
            expanded.push(path.clone());
        }
    }
    expanded
}

fn files_within(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    found
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PDF))
}

#[cfg(test)]
mod tests;
