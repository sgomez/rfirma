//! **Lo que se decide al soltar ficheros en la ventana** (ID-67, ID-68, ID-70).
//!
//! El arrastre no pasa por el portal de ficheros: lo que llega son rutas, y
//! aquí se decide cuál de ellas —si alguna— se abre. La decisión es de este
//! lado y no de la ventana por lo mismo que el diálogo se abre desde Rust
//! (ID-63): **ninguna ruta del anfitrión cruza a la interfaz** (ADR-0011), así
//! que quien las mira tiene que estar aquí.
//!
//! # Qué llega de verdad al soltar
//!
//! Está medido, no supuesto, en `docs/research/arrastre-bajo-el-sandbox.md`.
//! El resumen que este módulo necesita: cuando el origen del arrastre habla el
//! portal `FileTransfer` —Nautilus y cualquier GTK moderno— llega una ruta
//! exportada en `/run/user/1000/doc/…` que **se lee desde cualquier carpeta**;
//! cuando no lo habla, llega la ruta del anfitrión tal cual, y bajo el sandbox
//! esa ruta solo existe si cae dentro de la carpeta de documentos. Esa última
//! combinación es el único fallo real, y es el que [`Dropped::Unreadable`]
//! nombra.
//!
//! # Ser PDF es tener la extensión, aquí
//!
//! La misma regla que el diálogo, que filtra con `add_filter("PDF", &["pdf"])`
//! (ID-64): si el explorador de archivos deja elegir por extensión, soltar no
//! puede ser más estricto. Mirar los bytes es el trabajo de
//! [`crate::signing::AdmissibleDocument`], que sigue corriendo antes de pedir
//! el PIN y sigue siendo quien rechaza un `.pdf` que no lo es.

use std::path::{Path, PathBuf};

/// Qué hacer con lo que se acaba de soltar.
///
/// `ignored` es **cuántos ficheros más** venían en el mismo gesto, sea cual sea
/// el desenlace: la aplicación firma de uno en uno y callarse los demás es el
/// silencio que el #81 viene a quitar (ID-70).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// El primer PDF soltado, que además se deja leer.
    Opened { path: PathBuf, ignored: usize },
    /// Ninguno de los ficheros soltados es un PDF.
    NotAPdf { ignored: usize },
    /// El primer PDF soltado está donde el sandbox no llega.
    Unreadable { detail: String, ignored: usize },
    /// No se ha soltado ningún fichero. No es un fallo y no se cuenta.
    Nothing,
}

/// La extensión que hace que un fichero cuente como PDF, mayúsculas aparte.
const PDF: &str = "pdf";

/// El primer PDF de los soltados, si se puede leer.
///
/// **No se prueba con el siguiente** cuando el primero no se deja leer, y es a
/// propósito: pasar al segundo abriría un documento que la persona no eligió
/// primero y encima taparía el motivo por el que el suyo no se abrió.
pub fn first_pdf(paths: &[PathBuf]) -> Dropped {
    let ignored = paths.len().saturating_sub(1);
    let Some(first) = paths.iter().find(|path| is_pdf(path)) else {
        return if paths.is_empty() {
            Dropped::Nothing
        } else {
            Dropped::NotAPdf { ignored }
        };
    };
    match std::fs::File::open(first) {
        Ok(_) => Dropped::Opened {
            path: first.clone(),
            ignored,
        },
        Err(error) => Dropped::Unreadable {
            detail: error.to_string(),
            ignored,
        },
    }
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PDF))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: la decisión es una lista de rutas y un `open`, así que corre
    /// en el carril rápido. Lo que no se prueba aquí es el arrastre de verdad:
    /// en el CI no hay escritorio, ni portal, ni quien arrastre.
    fn a_temporary_pdf(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("rfirma-dropped-{name}"));
        std::fs::write(&path, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
        path
    }

    /// Una ruta que **no existe dentro del sandbox**, que es exactamente la
    /// forma del fallo medido: el fichero está en el anfitrión y aquí da
    /// `ENOENT`.
    fn a_path_the_sandbox_cannot_reach() -> PathBuf {
        std::env::temp_dir().join("rfirma-dropped-no-existe/contrato.pdf")
    }

    #[test]
    fn a_single_readable_pdf_is_the_one_that_opens() {
        let pdf = a_temporary_pdf("solo.pdf");

        assert_eq!(
            first_pdf(std::slice::from_ref(&pdf)),
            Dropped::Opened {
                path: pdf,
                ignored: 0
            }
        );
    }

    #[test]
    fn something_that_is_not_a_pdf_is_told_and_nothing_opens() {
        let other = a_temporary_pdf("hoja.ods");

        assert_eq!(first_pdf(&[other]), Dropped::NotAPdf { ignored: 0 });
    }

    /// ID-70: se abre el primero que sea un PDF, aunque no sea el primero que
    /// se soltó, y los demás se cuentan para poder decirlo.
    #[test]
    fn the_first_pdf_of_several_files_opens_and_the_rest_are_counted() {
        let other = a_temporary_pdf("hoja.ods");
        let pdf = a_temporary_pdf("factura.pdf");
        let another = a_temporary_pdf("contrato.pdf");

        assert_eq!(
            first_pdf(&[other, pdf.clone(), another]),
            Dropped::Opened {
                path: pdf,
                ignored: 2
            }
        );
    }

    #[test]
    fn a_pdf_the_sandbox_cannot_read_is_a_failure_with_its_raw_detail() {
        let unreachable = a_path_the_sandbox_cannot_reach();

        let Dropped::Unreadable { detail, ignored } = first_pdf(&[unreachable]) else {
            panic!("un PDF que no se puede abrir tiene que contarse como tal");
        };

        assert_eq!(ignored, 0);
        assert!(!detail.is_empty(), "el detalle crudo no se pierde (ID-29)");
    }

    /// Y no se prueba con el siguiente: el segundo PDF no es el que se eligió.
    #[test]
    fn an_unreadable_first_pdf_does_not_fall_through_to_the_next_one() {
        let readable = a_temporary_pdf("segundo.pdf");

        let dropped = first_pdf(&[a_path_the_sandbox_cannot_reach(), readable]);

        assert!(matches!(dropped, Dropped::Unreadable { ignored: 1, .. }));
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

    /// La ruta exportada por el portal es una ruta como otra cualquiera: se
    /// mira su extensión igual, y el nombre sigue estando en el último
    /// segmento.
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
