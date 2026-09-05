//! **Lo que se decide de los ficheros que llegan de fuera** (ID-67, ID-68,
//! ID-70, ID-157, ID-158, ID-306).
//!
//! Llegan por dos gestos —soltarlos en la ventana y nombrarlos en la línea de
//! órdenes— y la regla es una sola: el primer PDF que se deje leer es el que se
//! abre. Que la invocación se decida aquí y no en un módulo propio es el ID-158
//! escrito en código: un argumento que no es un PDF legible **no arranca
//! ningún modo especial**, cuenta exactamente lo mismo que soltarlo dentro.
//!
//! El arrastre no pasa por el portal de ficheros: lo que llega son rutas, y
//! aquí se decide cuáles de ellas —si alguna— entran. La decisión es de este
//! lado y no de la ventana por lo mismo que el diálogo se abre desde Rust
//! (ID-63): **ninguna ruta del anfitrión cruza a la interfaz** (ADR-0011), así
//! que quien las mira tiene que estar aquí.
//!
//! # Los N PDF entran en Recientes, y solo el primero se abre (ID-306)
//!
//! Soltar varios PDF a la vez ya no calla a los que no fueron el primero: el
//! primero se abre en el visor y **todos los demás que también sean un PDF
//! legible** entran igual en Recientes, sin abrirse. Es la versión ligera de
//! la ficha 11: sin cola y sin firma encadenada, solo una fila más por cada
//! documento.
//!
//! Una carpeta soltada se recorre —**un solo nivel**, no sus subcarpetas— y sus
//! ficheros se tratan como si se hubieran soltado uno a uno: los que sean PDF
//! entran, y los que no, se cuentan como descartados igual que cualquier otro
//! fichero que no lo sea.
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
/// `discarded` es cuántos ficheros más venían en el mismo gesto —sueltos
/// directamente o encontrados al recorrer una carpeta— **y no han entrado en
/// ningún sitio**: la variante ya dice por qué (ID-306).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// El primer PDF soltado que se deja leer, que es el que se abre en el
    /// visor. `also_entering` es el resto de PDF del mismo gesto: entran
    /// igual en Recientes, pero sin abrirse.
    Opened {
        path: PathBuf,
        also_entering: Vec<PathBuf>,
        discarded: usize,
    },
    /// Ninguno de los ficheros soltados —ni los que traía una carpeta
    /// recorrida— es un PDF.
    NotAPdf { discarded: usize },
    /// El primer PDF soltado está donde el sandbox no llega.
    Unreadable { detail: String, discarded: usize },
    /// No se ha soltado ningún fichero. No es un fallo y no se cuenta.
    Nothing,
}

/// La extensión que hace que un fichero cuente como PDF, mayúsculas aparte.
const PDF: &str = "pdf";

/// El primer PDF de los soltados, si se puede leer, y qué hacer con el resto.
///
/// **No se prueba con el siguiente** cuando el primero no se deja leer, y es a
/// propósito: pasar al segundo abriría un documento que la persona no eligió
/// primero y encima taparía el motivo por el que el suyo no se abrió. Los
/// demás PDF —si el primero sí se abrió— entran en Recientes igualmente: ver
/// [`Dropped::Opened`].
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

/// **El PDF que trae la invocación**, si es que trae alguno (ID-157, ID-158).
///
/// `command_line` es la línea de órdenes entera, con el ejecutable delante: es
/// lo que dan tanto `std::env::args` como la segunda instancia, y descartarlo
/// aquí evita que cada llamante se acuerde de hacerlo.
///
/// La invocación es **un argumento posicional desnudo** (ID-157), así que lo
/// que empieza por `-` no es una ruta y no se mira; no cierra la puerta a
/// banderas ni a subcomandos más adelante, solo dice que hoy no los hay.
///
/// `from` es la carpeta desde la que se invocó, y **hace falta**: una segunda
/// invocación se decide dentro del proceso que ya estaba abierto, cuya carpeta
/// de trabajo es otra, y una ruta relativa resuelta contra la suya abriría un
/// fichero distinto o ninguno. Una ruta absoluta pasa intacta por
/// [`Path::join`].
pub fn invoked_pdf(command_line: &[String], from: &Path) -> Dropped {
    let paths: Vec<PathBuf> = command_line
        .iter()
        .skip(1)
        .filter(|argument| !argument.starts_with('-'))
        .map(|argument| from.join(argument))
        .collect();
    first_pdf(&paths)
}

/// Sustituye cada carpeta soltada por los ficheros que tiene dentro (ID-306).
///
/// **Un solo nivel**: no se entra en las subcarpetas de una carpeta soltada,
/// así que una subcarpeta se ignora en silencio, igual que hoy se ignoraría
/// cualquier ruta que ni siquiera fuera un fichero. Los ficheros que trae se
/// dejan mezclados —PDF y no PDF— y es [`first_pdf`] quien decide cuáles
/// entran y cuáles se cuentan como descartados, exactamente igual que con lo
/// soltado directamente.
///
/// El orden es alfabético dentro de cada carpeta, para que el resultado no
/// dependa del orden en que el sistema de ficheros entregue sus entradas.
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

/// Los ficheros de primer nivel dentro de una carpeta, en orden alfabético.
///
/// Una carpeta que no se puede leer no es un fallo del gesto entero: se cuenta
/// como si no hubiera traído ningún fichero.
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

    /// Una carpeta temporal vacía, lista para que la prueba deje caer dentro
    /// lo que necesite.
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

    /// ID-70/ID-306: se abre el primero que sea un PDF, aunque no sea el
    /// primero que se soltó, y los que no lo son se cuentan como descartados.
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

    /// ID-306: el que ha abierto la ventana no es el único que cuenta. Cada
    /// PDF que vino en el mismo gesto entra igual en Recientes.
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
        assert!(!detail.is_empty(), "el detalle crudo no se pierde (ID-29)");
    }

    /// Y no se prueba con el siguiente: el segundo PDF no es el que se eligió.
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

    /// ID-306: una carpeta soltada se recorre y de ella solo cuentan sus PDF.
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

    /// Una carpeta sin ningún PDF dentro se cuenta igual que un fichero que no
    /// lo sea: nada que abrir, y lo de dentro, descartado.
    #[test]
    fn a_dropped_folder_with_no_pdf_inside_opens_nothing() {
        let folder = a_temporary_folder("vacia-de-pdf");
        std::fs::write(folder.join("nota.txt"), b"no es un pdf")
            .expect("se puede escribir en el temporal");

        assert_eq!(first_pdf(&[folder]), Dropped::NotAPdf { discarded: 1 });
    }

    /// Solo se recorre **un nivel**: una subcarpeta dentro de la carpeta
    /// soltada se ignora en silencio, no se cuenta como descartada.
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

    /// El caso entero del ID-157: un argumento posicional desnudo y nada más.
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

    /// Una ruta relativa se resuelve contra la carpeta desde la que se invocó,
    /// que en la segunda instancia **no** es la del proceso que la atiende.
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

    /// ID-158: lo que no es un PDF legible se cuenta igual que si se hubiera
    /// soltado, y no arranca ningún modo especial.
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

    /// Sin argumentos no hay invocación con documento: es arrancar la ventana.
    #[test]
    fn invoking_with_no_arguments_brings_no_document() {
        assert_eq!(
            invoked_pdf(&["rfirma".to_owned()], Path::new("/")),
            Dropped::Nothing
        );
    }

    /// Una bandera no es una ruta: ni se abre ni cuenta como fichero ignorado.
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
