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

/// El PDF elegido entre los candidatos, a la espera de saber si se deja leer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// El primer PDF, el resto que entran en recientes y cuántos candidatos había.
    Pdf {
        path: PathBuf,
        also_entering: Vec<PathBuf>,
        discarded: usize,
        candidates: usize,
    },
    /// Ninguno de los candidatos es un PDF.
    NotAPdf { discarded: usize },
    /// No había ningún candidato.
    Nothing,
}

/// Elige el primer PDF entre los candidatos ya expandidos.
pub fn first_pdf(candidates: &[PathBuf]) -> Choice {
    if candidates.is_empty() {
        return Choice::Nothing;
    }
    let discarded = candidates.iter().filter(|path| !is_pdf(path)).count();
    let mut pdfs = candidates.iter().filter(|path| is_pdf(path));
    let Some(first) = pdfs.next() else {
        return Choice::NotAPdf { discarded };
    };
    Choice::Pdf {
        path: first.clone(),
        also_entering: pdfs.cloned().collect(),
        discarded,
        candidates: candidates.len(),
    }
}

/// Resuelve la elección con lo que dijo el disco al intentar abrir el PDF elegido.
pub fn resolved(choice: Choice, readable: Result<(), String>) -> Dropped {
    match choice {
        Choice::Nothing => Dropped::Nothing,
        Choice::NotAPdf { discarded } => Dropped::NotAPdf { discarded },
        Choice::Pdf {
            path,
            also_entering,
            discarded,
            candidates,
        } => match readable {
            Ok(()) => Dropped::Opened {
                path,
                also_entering,
                discarded,
            },
            Err(detail) => Dropped::Unreadable {
                detail,
                discarded: candidates - 1,
            },
        },
    }
}

/// Las rutas que la línea de órdenes propone como documentos.
pub fn invoked_paths(command_line: &[String], from: &Path) -> Vec<PathBuf> {
    command_line
        .iter()
        .skip(1)
        .filter(|argument| !argument.starts_with('-'))
        .map(|argument| from.join(argument))
        .collect()
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PDF))
}

#[cfg(test)]
mod tests;
