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
mod tests {
    use super::*;

    fn a_temporary_pdf(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("rfirma-dropped-{name}"));
        std::fs::write(&path, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
        path
    }

    fn a_path_the_sandbox_cannot_reach() -> PathBuf {
        std::env::temp_dir().join("rfirma-dropped-no-existe/contrato.pdf")
    }

    fn a_temporary_folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("rfirma-dropped-folder-{name}"));
        std::fs::create_dir_all(&path).expect("se puede crear el temporal");
        path
    }

    #[test]
    fn a_single_readable_pdf_is_the_one_that_opens() {
        let pdf = a_temporary_pdf("solo.pdf");

        assert_eq!(
            first_pdf(std::slice::from_ref(&pdf)),
            Dropped::Opened {
                path: pdf,
                also_entering: Vec::new(),
                discarded: 0,
            }
        );
    }

    #[test]
    fn something_that_is_not_a_pdf_is_told_and_nothing_opens() {
        let other = a_temporary_pdf("hoja.ods");

        assert_eq!(first_pdf(&[other]), Dropped::NotAPdf { discarded: 1 });
    }

    #[test]
    fn the_first_pdf_of_several_files_opens_and_the_rest_are_counted() {
        let other = a_temporary_pdf("hoja.ods");
        let pdf = a_temporary_pdf("factura.pdf");
        let another = a_temporary_pdf("contrato.pdf");

        assert_eq!(
            first_pdf(&[other, pdf.clone(), another.clone()]),
            Dropped::Opened {
                path: pdf,
                also_entering: vec![another],
                discarded: 1,
            }
        );
    }

    #[test]
    fn every_pdf_dropped_together_also_enters_and_none_is_silenced() {
        let first = a_temporary_pdf("primero.pdf");
        let second = a_temporary_pdf("segundo.pdf");
        let third = a_temporary_pdf("tercero.pdf");

        let dropped = first_pdf(&[first.clone(), second.clone(), third.clone()]);

        assert_eq!(
            dropped,
            Dropped::Opened {
                path: first,
                also_entering: vec![second, third],
                discarded: 0,
            }
        );
    }

    #[test]
    fn a_pdf_the_sandbox_cannot_read_is_a_failure_with_its_raw_detail() {
        let unreachable = a_path_the_sandbox_cannot_reach();

        let Dropped::Unreadable { detail, discarded } = first_pdf(&[unreachable]) else {
            panic!("un PDF que no se puede abrir tiene que contarse como tal");
        };

        assert_eq!(discarded, 0);
        assert!(!detail.is_empty(), "el detalle crudo no se pierde");
    }

    #[test]
    fn an_unreadable_first_pdf_does_not_fall_through_to_the_next_one() {
        let readable = a_temporary_pdf("segundo.pdf");

        let dropped = first_pdf(&[a_path_the_sandbox_cannot_reach(), readable]);

        assert!(matches!(dropped, Dropped::Unreadable { discarded: 1, .. }));
    }

    #[test]
    fn the_extension_is_read_without_minding_the_case() {
        let shouted = a_temporary_pdf("CONTRATO.PDF");

        assert!(matches!(first_pdf(&[shouted]), Dropped::Opened { .. }));
    }

    #[test]
    fn dropping_nothing_is_not_a_failure() {
        assert_eq!(first_pdf(&[]), Dropped::Nothing);
    }

    #[test]
    fn a_dropped_folder_is_walked_and_only_its_pdfs_enter() {
        let folder = a_temporary_folder("mixta");
        let pdf = folder.join("factura.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
        std::fs::write(folder.join("nota.txt"), b"no es un pdf")
            .expect("se puede escribir en el temporal");

        assert_eq!(
            first_pdf(&[folder]),
            Dropped::Opened {
                path: pdf,
                also_entering: Vec::new(),
                discarded: 1,
            }
        );
    }

    #[test]
    fn a_dropped_folder_with_no_pdf_inside_opens_nothing() {
        let folder = a_temporary_folder("vacia-de-pdf");
        std::fs::write(folder.join("nota.txt"), b"no es un pdf")
            .expect("se puede escribir en el temporal");

        assert_eq!(first_pdf(&[folder]), Dropped::NotAPdf { discarded: 1 });
    }

    #[test]
    fn a_subfolder_of_a_dropped_folder_is_not_walked_into() {
        let folder = a_temporary_folder("con-subcarpeta");
        let pdf = folder.join("factura.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
        let inner = folder.join("subcarpeta");
        std::fs::create_dir_all(&inner).expect("se puede crear el temporal");
        std::fs::write(inner.join("otra.pdf"), b"%PDF-1.4\n")
            .expect("se puede escribir en el temporal");

        assert_eq!(
            first_pdf(&[folder]),
            Dropped::Opened {
                path: pdf,
                also_entering: Vec::new(),
                discarded: 0,
            }
        );
    }

    #[test]
    fn a_bare_positional_argument_is_the_document_that_opens() {
        let pdf = a_temporary_pdf("invocado.pdf");

        let invoked = invoked_pdf(
            &["rfirma".to_owned(), pdf.display().to_string()],
            Path::new("/"),
        );

        assert_eq!(
            invoked,
            Dropped::Opened {
                path: pdf,
                also_entering: Vec::new(),
                discarded: 0,
            }
        );
    }

    #[test]
    fn a_relative_argument_is_resolved_against_the_folder_it_was_invoked_from() {
        let pdf = a_temporary_pdf("relativo.pdf");
        let folder = pdf.parent().expect("el temporal tiene carpeta").to_owned();

        let invoked = invoked_pdf(
            &[
                "rfirma".to_owned(),
                "rfirma-dropped-relativo.pdf".to_owned(),
            ],
            &folder,
        );

        assert!(matches!(invoked, Dropped::Opened { .. }));
    }

    #[test]
    fn an_argument_that_is_not_a_pdf_is_told_just_like_a_dropped_one() {
        let other = a_temporary_pdf("hoja-invocada.ods");

        assert_eq!(
            invoked_pdf(
                &["rfirma".to_owned(), other.display().to_string()],
                Path::new("/")
            ),
            Dropped::NotAPdf { discarded: 1 }
        );
    }

    #[test]
    fn invoking_with_no_arguments_brings_no_document() {
        assert_eq!(
            invoked_pdf(&["rfirma".to_owned()], Path::new("/")),
            Dropped::Nothing
        );
    }

    #[test]
    fn a_flag_is_not_a_path_and_does_not_count() {
        let pdf = a_temporary_pdf("con-bandera.pdf");

        let invoked = invoked_pdf(
            &[
                "rfirma".to_owned(),
                "--algo".to_owned(),
                pdf.display().to_string(),
            ],
            Path::new("/"),
        );

        assert_eq!(
            invoked,
            Dropped::Opened {
                path: pdf,
                also_entering: Vec::new(),
                discarded: 0,
            }
        );
    }

    #[test]
    fn a_path_exported_by_the_portal_is_a_path_like_any_other() {
        let exported = PathBuf::from("/run/user/1000/doc/1e20dd88/contrato.pdf");

        assert!(is_pdf(&exported));
        assert_eq!(
            exported.file_name().and_then(|name| name.to_str()),
            Some("contrato.pdf")
        );
    }
}
