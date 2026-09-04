//! **Firmar empieza fuera: la invocación con un documento** (ID-157…ID-160).
//!
//! `rfirma /ruta/documento.pdf` no es otra pantalla ni otro modo: es la misma
//! ventana en el mismo estado en que la deja arrastrar un PDF (ID-159), así que
//! lo que este módulo hace es traducir una línea de órdenes a lo mismo que
//! emite un arrastre y dejarla caer por el mismo sitio.
//!
//! # Por qué hay dos casos de uso y no uno
//!
//! Porque no son el mismo momento. La **primera** invocación abre la ventana, y
//! entonces no hay nadie escuchando todavía: lo que trae se guarda en
//! [`PendingInvocation`] y la ventana lo recoge cuando ya está montada. La
//! **segunda** cae sobre una ventana viva —hay instancia única (ID-160)— y ahí
//! sí hay a quién contárselo, pero aparece la única excepción del hito: **con
//! una sesión de firma viva no se sustituye nada**, porque es el único estado
//! donde perder el hilo cuesta un PIN.
//!
//! Si se sustituye se pierde la colocación de un recuadro, que no significa
//! nada en otro documento; por eso no se pregunta.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::app::documents;
use crate::commands::views::DroppedDocumentView;
use crate::memory::OpenedDocuments;

/// Una invocación, tal y como llegó: la línea de órdenes y desde dónde se
/// corrió.
///
/// Las dos cosas viajan juntas porque una ruta relativa no significa nada sin
/// la segunda, y la segunda instancia se atiende dentro de un proceso cuya
/// carpeta de trabajo es otra.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invocation {
    /// La línea de órdenes entera, con el ejecutable delante.
    pub command_line: Vec<String>,
    /// La carpeta de trabajo desde la que se invocó.
    pub folder: PathBuf,
}

impl Invocation {
    /// La invocación que arrancó **este** proceso.
    ///
    /// Un `HOME` o una carpeta de trabajo que ya no existen no son motivo para
    /// no abrir la ventana: sin carpeta, solo dejan de resolverse las rutas
    /// relativas, que es justo lo que ya no se podía hacer.
    pub fn of_this_process() -> Self {
        Self {
            command_line: std::env::args().collect(),
            folder: std::env::current_dir().unwrap_or_default(),
        }
    }
}

/// **Caso de uso.** Qué abre una invocación, contado como la ventana lo
/// entiende.
///
/// Devuelve `None` cuando la invocación no nombra ningún fichero —arrancar la
/// aplicación a secas—, igual que soltar nada no emite ningún evento. Un
/// argumento que no es un PDF legible **sí** devuelve algo: la ventana normal
/// diciéndolo (ID-158).
pub fn invoked_document(
    invocation: &Invocation,
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    documents::told_as_dropped(
        crate::dropped::invoked_pdf(&invocation.command_line, &invocation.folder),
        opened,
    )
}

/// **Caso de uso.** Qué hace una segunda invocación sobre la ventana que ya
/// estaba abierta (ID-160).
///
/// **Sustituye lo que hubiera, sin preguntar**, salvo con una sesión de firma
/// viva: entonces no devuelve nada y no se toca nada. `signing_is_live` llega
/// resuelto y no como la sesión entera a propósito — lo que decide aquí es
/// «hay una firma a medias, sí o no», y pedir la sesión ataría esta regla al
/// ciclo trifásico para leer un booleano.
pub fn second_invocation(
    invocation: &Invocation,
    opened: &OpenedDocuments,
    signing_is_live: bool,
) -> Option<DroppedDocumentView> {
    if signing_is_live {
        return None;
    }
    invoked_document(invocation, opened)
}

/// **Lo que traía la invocación que abrió la ventana**, hasta que la ventana lo
/// pide.
///
/// Existe por un desajuste de tiempos y no por una decisión: el documento se
/// conoce al arrancar el proceso y no hay quien lo escuche hasta que el frontal
/// se monta. Se **consume**: una ventana que vuelva a preguntar no reabre el
/// documento de hace media hora.
#[derive(Default)]
pub struct PendingInvocation(Mutex<Option<Invocation>>);

impl PendingInvocation {
    /// La invocación que queda pendiente de recoger.
    pub fn of(invocation: Invocation) -> Self {
        Self(Mutex::new(Some(invocation)))
    }

    /// La recoge, y deja de estar pendiente.
    pub fn take(&self) -> Option<Invocation> {
        crate::app::lock(&self.0).take()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// **Grada A**: una línea de órdenes y un fichero temporal. Ni token, ni
    /// puente, ni ventana.
    fn a_temporary_pdf(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("rfirma-invocation-{name}"));
        std::fs::write(&path, b"%PDF-1.4\n").expect("se puede escribir en el temporal");
        path
    }

    fn invoked_with(path: &Path) -> Invocation {
        Invocation {
            command_line: vec!["rfirma".to_owned(), path.display().to_string()],
            folder: PathBuf::from("/"),
        }
    }

    #[test]
    fn a_pdf_named_in_the_command_line_opens_like_a_dropped_one() {
        let pdf = a_temporary_pdf("contrato.pdf");
        let opened = OpenedDocuments::new();

        let view = invoked_document(&invoked_with(&pdf), &opened).expect("algo trae");

        assert!(view.failure.is_none(), "un PDF legible se abre y no avisa");
        assert_eq!(view.ignored, 0);
        let document = view.document.expect("y el documento cruza ya apuntado");
        assert_eq!(document.name, "rfirma-invocation-contrato.pdf");
    }

    /// ID-158: no hay modo especial que arrancar, hay una ventana que lo dice.
    #[test]
    fn an_argument_that_is_not_a_pdf_opens_the_normal_window_and_says_so() {
        let other = a_temporary_pdf("hoja.ods");

        let view =
            invoked_document(&invoked_with(&other), &OpenedDocuments::new()).expect("algo trae");

        assert!(view.document.is_none(), "no se abre ningun documento");
        assert_eq!(
            view.failure.expect("y se dice por que").situation,
            "notAPdf"
        );
    }

    #[test]
    fn invoking_without_a_document_is_just_opening_the_application() {
        let invocation = Invocation {
            command_line: vec!["rfirma".to_owned()],
            folder: PathBuf::from("/"),
        };

        assert_eq!(invoked_document(&invocation, &OpenedDocuments::new()), None);
    }

    /// **TD-46, primer caso.** Una segunda invocación con un documento
    /// sustituye el que había, y no pregunta nada.
    #[test]
    fn a_second_invocation_with_a_document_replaces_the_one_that_was_there() {
        let pdf = a_temporary_pdf("segundo.pdf");
        let opened = OpenedDocuments::new();

        let view = second_invocation(&invoked_with(&pdf), &opened, false).expect("sustituye");

        assert!(
            view.document.is_some(),
            "el documento nuevo es el que queda"
        );
    }

    /// **TD-46, segundo caso.** Con una firma a medias no se sustituye nada:
    /// perder el hilo ahí cuesta un PIN (ID-160).
    #[test]
    fn a_second_invocation_replaces_nothing_while_a_signing_session_is_live() {
        let pdf = a_temporary_pdf("mientras-firmo.pdf");

        assert_eq!(
            second_invocation(&invoked_with(&pdf), &OpenedDocuments::new(), true),
            None
        );
    }

    /// Y tampoco se cuela el aviso: con la firma viva no llega nada, ni
    /// documento ni situación.
    #[test]
    fn not_even_a_notice_reaches_the_window_while_a_signing_session_is_live() {
        let other = a_temporary_pdf("hoja-mientras-firmo.ods");

        assert_eq!(
            second_invocation(&invoked_with(&other), &OpenedDocuments::new(), true),
            None
        );
    }

    #[test]
    fn the_pending_invocation_is_handed_over_once_and_only_once() {
        let pdf = a_temporary_pdf("pendiente.pdf");
        let pending = PendingInvocation::of(invoked_with(&pdf));

        assert_eq!(pending.take(), Some(invoked_with(&pdf)));
        assert_eq!(pending.take(), None);
    }

    #[test]
    fn a_window_opened_with_nothing_pending_has_nothing_to_pick_up() {
        assert_eq!(PendingInvocation::default().take(), None);
    }
}
