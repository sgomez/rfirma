//! Gestión de la línea de órdenes en el arranque e invocación desde el escritorio (ADR-0010, ADR-0015).

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::app::documents;
use crate::commands::views::DroppedDocumentView;
use crate::memory::OpenedDocuments;
use crate::protocol::AfirmaUrl;

/// Invocación recibida con sus argumentos y carpeta de trabajo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invocation {
    /// Argumentos de la línea de órdenes.
    pub command_line: Vec<String>,
    /// Directorio de trabajo en el momento de la invocación.
    pub folder: PathBuf,
}

impl Invocation {
    /// Obtiene la invocación correspondiente al proceso actual.
    pub fn of_this_process() -> Self {
        Self {
            command_line: std::env::args_os()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
            folder: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// Extrae la URL con esquema afirma:// si la invocación la incluye.
    pub fn site_launch(&self) -> Option<&str> {
        self.command_line
            .iter()
            .skip(1)
            .map(String::as_str)
            .find(|argument| AfirmaUrl::is_a_protocol_url(argument))
    }
}

/// Resultado del análisis de codificación de los argumentos del proceso.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arguments {
    /// Argumentos con codificación UTF-8 válida.
    Readable,
    /// Argumentos que requieren reejecución con sustitución de caracteres no válidos.
    RerunWith(Vec<String>),
}

/// Analiza los argumentos de ejecución para evitar fallos de codificación.
pub fn arguments_before_the_single_instance<I>(arguments: I) -> Arguments
where
    I: IntoIterator<Item = OsString>,
{
    let arguments: Vec<OsString> = arguments.into_iter().collect();
    if arguments.iter().all(|argument| argument.to_str().is_some()) {
        return Arguments::Readable;
    }
    Arguments::RerunWith(
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect(),
    )
}

/// Asegura que los argumentos de la línea de órdenes tengan codificación UTF-8 válida.
pub fn make_the_command_line_readable() {
    let Arguments::RerunWith(arguments) = arguments_before_the_single_instance(std::env::args_os())
    else {
        return;
    };
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            eprintln!(
                "rfirma: no se puede releer la línea de órdenes ilegible \
                 ({error}); el arranque sigue con ella tal cual"
            );
            return;
        }
    };
    match std::process::Command::new(executable)
        .args(arguments.iter().skip(1))
        .spawn()
    {
        Ok(_) => std::process::exit(0),
        Err(error) => eprintln!(
            "rfirma: no se puede volver a arrancar con la línea de órdenes ya \
             legible ({error}); el arranque sigue con ella tal cual"
        ),
    }
}

/// Formas aceptadas del parámetro de ayuda.
pub const HELP_FLAGS: [&str; 3] = ["--help", "-help", "-h"];

/// Texto informativo mostrado en la ayuda por consola.
pub const HELP: &str = "\
rfirma — firma y cofirma de documentos PDF en PAdES.

Uso:
  rfirma [documento…]
  rfirma «afirma://…»
  rfirma --help

Argumentos:
  documento           Ruta de un PDF: se abre en la ventana, listo para firmar.
                      Lo que no sea un PDF abre la ventana igual y lo dice.
  afirma://…          La llamada de una sede electrónica. La entrega el
                      navegador a través del manejador del esquema; a mano,
                      sirve para probar.

Opciones:
  -h, -help, --help   Muestra esta ayuda y termina.

Lo que rFirma atiende de una sede (protocolo 4, sobre wss:// en 127.0.0.1):
  websocket           Abre el canal en uno de los puertos que sortea la sede.
  echo                Comprobación de vida.
  selectcert          Elegir certificado, consentido por la persona.
  sign                Firma PAdES de un PDF.
  cosign              Cofirma PAdES de un PDF.
  countersign, save y signandsave se rechazan con su código del catálogo.

Compatibilidad con AutoFirma:
  rFirma la sustituye en la llamada desde el navegador —el esquema afirma://—,
  que es como la usan las sedes electrónicas. NO implementa su línea de órdenes
  de firma desatendida, así que ninguna de estas órdenes ni de estos parámetros
  existe aquí:

    órdenes      sign, cosign, countersign, listaliases, verify, batchsign
    parámetros   -i, -o, -alias, -filter, -store, -format, -password,
                 -algorithm, -config, -operation, -gui, -certgui, -preurl,
                 -posturl, -hformat, -halgorithm, -r, -xml

  Toda firma la consiente la persona delante de la ventana. No hay modo
  desatendido y no está previsto que lo haya.
";

/// Determina si los argumentos de ejecución solicitan la visualización de la ayuda.
pub fn help_was_asked_for<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments
        .into_iter()
        .skip(1)
        .any(|argument| HELP_FLAGS.contains(&argument.as_ref()))
}

/// Procesa la invocación como apertura de documento para la ventana principal.
pub fn invoked_document(
    invocation: &Invocation,
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    if invocation.site_launch().is_some() {
        return None;
    }
    documents::told_as_dropped(
        crate::dropped::invoked_pdf(&invocation.command_line, &invocation.folder),
        opened,
    )
}

/// Destino de una segunda invocación recibida con la aplicación ya en marcha.
#[derive(Debug, PartialEq, Eq)]
pub enum SecondInvocation {
    /// Se ignora la segunda invocación.
    NothingHappens,
    /// Sustituye el documento activo por el nuevo.
    ReplacesWhatWasThere(Box<DroppedDocumentView>),
    /// Abre una ventana dedicada para atender el trámite de sede.
    OpensItsOwnWindow(String),
}

/// Determina la acción a tomar ante una segunda invocación del proceso.
pub fn second_invocation(
    invocation: &Invocation,
    opened: &OpenedDocuments,
    signing_is_live: bool,
) -> SecondInvocation {
    if let Some(url) = invocation.site_launch() {
        return SecondInvocation::OpensItsOwnWindow(url.to_owned());
    }
    if signing_is_live {
        return SecondInvocation::NothingHappens;
    }
    match invoked_document(invocation, opened) {
        Some(view) => SecondInvocation::ReplacesWhatWasThere(Box::new(view)),
        None => SecondInvocation::NothingHappens,
    }
}

/// Contenedor de la invocación inicial pendiente de consumo por la ventana.
#[derive(Default)]
pub struct PendingInvocation(Mutex<Option<Invocation>>);

impl PendingInvocation {
    /// Inicializa el contenedor con una invocación pendiente.
    pub fn of(invocation: Invocation) -> Self {
        Self(Mutex::new(Some(invocation)))
    }

    /// Extrae la invocación pendiente si aún no ha sido consumida.
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
        assert_eq!(view.discarded, 0);
        let document = view.document.expect("y el documento cruza ya apuntado");
        assert_eq!(document.name, "rfirma-invocation-contrato.pdf");
    }

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

    #[test]
    fn a_second_invocation_with_a_document_replaces_the_one_that_was_there() {
        let pdf = a_temporary_pdf("segundo.pdf");
        let opened = OpenedDocuments::new();

        let second = second_invocation(&invoked_with(&pdf), &opened, false);

        let SecondInvocation::ReplacesWhatWasThere(view) = second else {
            panic!("sustituye: {second:?}");
        };
        assert!(
            view.document.is_some(),
            "el documento nuevo es el que queda"
        );
    }

    #[test]
    fn a_second_invocation_replaces_nothing_while_a_signing_session_is_live() {
        let pdf = a_temporary_pdf("mientras-firmo.pdf");

        assert_eq!(
            second_invocation(&invoked_with(&pdf), &OpenedDocuments::new(), true),
            SecondInvocation::NothingHappens
        );
    }

    #[test]
    fn not_even_a_notice_reaches_the_window_while_a_signing_session_is_live() {
        let other = a_temporary_pdf("hoja-mientras-firmo.ods");

        assert_eq!(
            second_invocation(&invoked_with(&other), &OpenedDocuments::new(), true),
            SecondInvocation::NothingHappens
        );
    }

    #[test]
    fn a_site_launch_opens_its_own_window_and_replaces_nothing() {
        assert_eq!(
            second_invocation(
                &invoked_with_the_url(A_LAUNCH),
                &OpenedDocuments::new(),
                false
            ),
            SecondInvocation::OpensItsOwnWindow(A_LAUNCH.to_owned())
        );
    }

    #[test]
    fn a_live_signing_session_does_not_stop_a_site_launch() {
        assert_eq!(
            second_invocation(
                &invoked_with_the_url(A_LAUNCH),
                &OpenedDocuments::new(),
                true
            ),
            SecondInvocation::OpensItsOwnWindow(A_LAUNCH.to_owned())
        );
    }

    #[test]
    fn the_help_is_asked_for_with_any_of_its_three_flags_and_from_any_position() {
        for flag in ["--help", "-help", "-h"] {
            assert!(help_was_asked_for(["rfirma", flag]), "con {flag} sola");
            assert!(
                help_was_asked_for(["rfirma", "documento.pdf", flag]),
                "con {flag} detrás de un documento"
            );
        }
    }

    #[test]
    fn nothing_else_asks_for_the_help() {
        assert!(!help_was_asked_for(["rfirma"]));
        assert!(!help_was_asked_for(["rfirma", "documento.pdf"]));
        assert!(!help_was_asked_for([
            "rfirma",
            "afirma://websocket?ports=51000"
        ]));
        assert!(!help_was_asked_for(["rfirma", "--helpful"]));
        assert!(!help_was_asked_for(["-h"]), "el ejecutable no cuenta");
    }

    #[test]
    fn the_help_names_every_autofirma_command_and_parameter() {
        for command in [
            "sign",
            "cosign",
            "countersign",
            "listaliases",
            "verify",
            "batchsign",
        ] {
            assert!(HELP.contains(command), "falta la orden {command}");
        }
        for parameter in [
            "-i",
            "-o",
            "-alias",
            "-filter",
            "-store",
            "-format",
            "-password",
            "-algorithm",
            "-config",
            "-operation",
            "-gui",
            "-certgui",
            "-preurl",
            "-posturl",
            "-hformat",
            "-halgorithm",
            "-r",
            "-xml",
        ] {
            assert!(HELP.contains(parameter), "falta el parámetro {parameter}");
        }
    }

    #[test]
    fn the_help_names_the_three_ways_of_asking_for_it() {
        for flag in HELP_FLAGS {
            assert!(HELP.contains(flag), "falta {flag}");
        }
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

    const A_LAUNCH: &str =
        "afirma://websocket?ports=51000,51001&v=4&idsession=8jAkPZfRw2mQxN4TbYuL";

    fn invoked_with_the_url(url: &str) -> Invocation {
        Invocation {
            command_line: vec!["rfirma".to_owned(), url.to_owned()],
            folder: PathBuf::from("/"),
        }
    }

    #[test]
    fn a_site_url_is_not_treated_as_a_file_path() {
        let invocation = invoked_with_the_url(A_LAUNCH);

        assert_eq!(invocation.site_launch(), Some(A_LAUNCH));
        assert_eq!(invoked_document(&invocation, &OpenedDocuments::new()), None);
    }

    #[test]
    fn the_whole_url_survives_the_single_instance_path() {
        let invocation = Invocation {
            command_line: vec!["rfirma".to_owned(), A_LAUNCH.to_owned()],
            folder: PathBuf::from("/otra/carpeta"),
        };

        assert_eq!(invocation.site_launch(), Some(A_LAUNCH));
        assert_eq!(
            second_invocation(&invocation, &OpenedDocuments::new(), false),
            SecondInvocation::OpensItsOwnWindow(A_LAUNCH.to_owned()),
            "una invocación de sede no sustituye ningún documento: abre lo suyo"
        );
    }

    #[test]
    fn the_scheme_is_recognised_whatever_the_case_and_never_in_the_executable() {
        assert_eq!(
            invoked_with_the_url("AFIRMA://selectcert?ports=51000").site_launch(),
            Some("AFIRMA://selectcert?ports=51000")
        );
        assert_eq!(
            Invocation {
                command_line: vec!["afirma://rfirma".to_owned()],
                folder: PathBuf::from("/"),
            }
            .site_launch(),
            None
        );
    }

    #[test]
    fn an_argument_that_is_not_utf8_is_made_readable_before_the_plugin_reads_it() {
        use std::os::unix::ffi::OsStringExt as _;

        let unreadable = OsString::from_vec(vec![b'/', 0xff, b'.', b'p', b'd', b'f']);

        assert_eq!(
            arguments_before_the_single_instance(vec![OsString::from("rfirma"), unreadable]),
            Arguments::RerunWith(vec!["rfirma".to_owned(), "/\u{fffd}.pdf".to_owned()])
        );
    }

    #[test]
    fn a_readable_command_line_reruns_nothing() {
        assert_eq!(
            arguments_before_the_single_instance(vec![
                OsString::from("rfirma"),
                OsString::from(A_LAUNCH),
            ]),
            Arguments::Readable
        );
    }
}
