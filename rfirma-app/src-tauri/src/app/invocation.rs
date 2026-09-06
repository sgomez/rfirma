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
//!
//! # La invocación que no nombra un fichero
//!
//! Desde el hito de la sede hay una segunda forma de invocar: el navegador
//! abre `rfirma 'afirma://websocket?…'` al pulsar un enlace del esquema
//! (ID-234, ID-235). Esa cadena **no es una ruta**, así que no baja a
//! [`crate::dropped`]: se reconoce por el esquema —esté bien formada o no— y
//! sale por [`Invocation::site_launch`], que es por donde la recogerá el
//! trámite de sede. Lo que este módulo garantiza es que llega **entera**, con
//! su `?` y sus pares intactos, también por el camino de la segunda instancia.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::app::documents;
use crate::commands::views::DroppedDocumentView;
use crate::memory::OpenedDocuments;
use crate::protocol::AfirmaUrl;

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
            // `args()` **entra en pánico** con un argumento que no sea UTF-8
            // válido, y en Linux una ruta es una cadena de bytes: morir en el
            // arranque es lo contrario del ID-158. Una ruta ilegible sale por
            // `NotAPdf`, la misma puerta que los demás argumentos que no se
            // pueden abrir.
            command_line: std::env::args_os()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
            folder: std::env::current_dir().unwrap_or_default(),
        }
    }

    /// La URL `afirma://` que trae la invocación, si la trae (ID-235).
    ///
    /// Se reconoce **por el esquema y nada más**: una URL rota es igualmente
    /// de la sede, y lo que le toca después es un código del catálogo, no
    /// intentar abrirla como fichero. Se devuelve tal cual llegó —el
    /// escritorio la entrega entera en un solo argumento gracias al `%u` del
    /// ID-234— porque los pares se leen luego, en `protocol`.
    pub fn site_launch(&self) -> Option<&str> {
        self.command_line
            .iter()
            .skip(1)
            .map(String::as_str)
            .find(|argument| AfirmaUrl::is_a_protocol_url(argument))
    }
}

/// Qué hacer con los argumentos de este proceso **antes de montar nada**
/// (ID-236).
///
/// `tauri-plugin-single-instance` lee la línea de órdenes con
/// `std::env::args()`, que **entra en pánico** con un argumento que no sea
/// UTF-8 válido, y lo hace dentro de su propio `setup`: ni la protección de
/// [`Invocation::of_this_process`] (ID-158) ni el cierre de la segunda
/// invocación llegan a tiempo. La única forma de que no pase por ahí es que
/// para cuando el complemento mire, los argumentos ya sean legibles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arguments {
    /// Todos son UTF-8 válido: el complemento puede leerlos sin morirse.
    Readable,
    /// Alguno no lo es, y hay que volver a arrancar con estos otros, ya
    /// legibles, que dicen lo mismo con los bytes ilegibles sustituidos.
    RerunWith(Vec<String>),
}

/// Decide lo anterior sobre una línea de órdenes cualquiera.
///
/// La sustitución es la misma de [`Invocation::of_this_process`]: un argumento
/// ilegible acaba saliendo por `NotAPdf`, la puerta de siempre (ID-158). Una
/// URL `afirma://` es UTF-8 por construcción, así que este camino no la toca.
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

/// Deja la línea de órdenes de este proceso **legible**, volviendo a arrancar
/// si hace falta (ID-236).
///
/// Se llama lo primero de todo, antes de construir la aplicación: para cuando
/// el complemento de instancia única mire, los argumentos ya son UTF-8 y su
/// `std::env::args()` no tiene con qué entrar en pánico. El proceso nuevo no
/// vuelve a pasar por aquí, porque una sustitución lossy siempre sale legible.
///
/// Vuelve a arrancar con `spawn` y no con el `exec` de POSIX porque el
/// condicional de sistema operativo solo puede vivir en
/// [`crate::paths`] (ID-35, ADR-0010) y `exec` es de `std::os::unix`. Si ni
/// siquiera se puede arrancar el proceso nuevo, se sigue: no abrir la ventana
/// es peor que arriesgarse a un argumento ilegible, que es lo raro de lo raro.
pub fn make_the_command_line_readable() {
    let Arguments::RerunWith(arguments) = arguments_before_the_single_instance(std::env::args_os())
    else {
        return;
    };
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            // Se sigue adelante a sabiendas: no abrir la ventana sería peor que
            // el pánico del complemento. Al menos queda dicho por qué.
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

/// Las tres formas de pedir la ayuda.
///
/// `--help` y `-h` son lo que espera quien viene de cualquier programa de
/// consola; `-help` es la que documenta AutoFirma, y se acepta para que quien
/// llega con su manual delante no se quede sin nada (ID-157).
pub const HELP_FLAGS: [&str; 3] = ["--help", "-help", "-h"];

/// La ayuda, tal y como sale por la salida estándar.
///
/// Dice **lo que rFirma acepta** y, detrás, lo que AutoFirma acepta y aquí no
/// existe. Lo segundo no es cortesía: la línea de órdenes de AutoFirma firma
/// sin preguntar, y rFirma no tiene modo desatendido ni lo va a tener, así que
/// callarse esas órdenes deja a quien las busca probándolas una a una.
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

/// Si la línea de órdenes pide la ayuda.
///
/// Se mira **antes de montar nada** y en cualquier posición: quien escribe
/// `rfirma documento.pdf --help` está pidiendo la ayuda, no una firma. El
/// ejecutable de delante no cuenta.
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

/// **Caso de uso.** Qué abre una invocación, contado como la ventana lo
/// entiende.
///
/// Devuelve `None` cuando la invocación no nombra ningún fichero —arrancar la
/// aplicación a secas—, igual que soltar nada no emite ningún evento. Un
/// argumento que no es un PDF legible **sí** devuelve algo: la ventana normal
/// diciéndolo (ID-158). La excepción es la URL `afirma://`, que no es un
/// fichero fallido sino otra cosa: devuelve `None` y se recoge por
/// [`Invocation::site_launch`] (ID-235).
pub fn invoked_document(
    invocation: &Invocation,
    opened: &OpenedDocuments,
) -> Option<DroppedDocumentView> {
    // Una invocación de la sede no abre ningún documento **ni avisa de nada**:
    // no es un argumento que no se pueda abrir, es un argumento que no es un
    // fichero (ID-235). Quien la atiende es el trámite de sede, leyéndola de
    // [`Invocation::site_launch`].
    if invocation.site_launch().is_some() {
        return None;
    }
    documents::told_as_dropped(
        crate::dropped::invoked_pdf(&invocation.command_line, &invocation.folder),
        opened,
    )
}

/// En qué queda una segunda invocación sobre la aplicación ya abierta (ID-160,
/// ID-279).
#[derive(Debug, PartialEq, Eq)]
pub enum SecondInvocation {
    /// **No pasa nada**: hay una firma a medias y no se sustituye (ID-160), o
    /// la invocación no nombraba nada que abrir.
    NothingHappens,
    /// Lo que la ventana tenía delante se sustituye por esto, sin preguntar.
    ReplacesWhatWasThere(Box<DroppedDocumentView>),
    /// **Una invocación de sede abre lo suyo** y no sustituye nada (ID-279).
    /// Lo que sale es la URL entera, que es lo que atiende
    /// [`super::site::attend_launch`].
    OpensItsOwnWindow(String),
}

/// **Caso de uso.** Qué hace una segunda invocación sobre la ventana que ya
/// estaba abierta (ID-160, ID-279).
///
/// **Sustituye lo que hubiera, sin preguntar**, con dos excepciones:
///
/// 1. Con una **sesión de firma viva** no se toca nada: es el único estado
///    donde perder el hilo cuesta un PIN. `signing_is_live` llega resuelto y no
///    como la sesión entera a propósito — lo que decide aquí es «hay una firma
///    a medias, sí o no», y pedir la sesión ataría esta regla al ciclo
///    trifásico para leer un booleano.
/// 2. Un **`afirma://` nunca sustituye**: abre su propia ventana (ID-279), y
///    por eso una firma local a medias tampoco lo detiene —no le quita a nadie
///    lo que tenía delante—. Si además se atiende o se rechaza es cosa del
///    trámite vivo (ID-280), y eso lo decide [`super::site::attend_launch`].
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
        assert_eq!(view.discarded, 0);
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

        let second = second_invocation(&invoked_with(&pdf), &opened, false);

        let SecondInvocation::ReplacesWhatWasThere(view) = second else {
            panic!("sustituye: {second:?}");
        };
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
            SecondInvocation::NothingHappens
        );
    }

    /// Y tampoco se cuela el aviso: con la firma viva no llega nada, ni
    /// documento ni situación.
    #[test]
    fn not_even_a_notice_reaches_the_window_while_a_signing_session_is_live() {
        let other = a_temporary_pdf("hoja-mientras-firmo.ods");

        assert_eq!(
            second_invocation(&invoked_with(&other), &OpenedDocuments::new(), true),
            SecondInvocation::NothingHappens
        );
    }

    /// **ID-279**: la invocación de una sede **abre lo suyo y no sustituye
    /// nada**, aunque la ventana tenga un documento delante.
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

    /// Y una firma local a medias tampoco la detiene: no le quita a nadie lo
    /// que tenía delante, así que la regla del ID-160 no la alcanza (ID-279).
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

    /// La ayuda se pide con las tres formas, y en cualquier posición: quien
    /// escribe `rfirma documento.pdf --help` quiere la ayuda, no una firma.
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

    /// Y no se pide con nada más. El nombre del ejecutable no cuenta: un
    /// binario que se llamara `-h` no dispararía la ayuda.
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

    /// El texto nombra **todo** lo que AutoFirma acepta por línea de órdenes,
    /// para que quien llegue con su manual delante no lo pruebe una a una.
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

    /// Y nombra las tres formas de pedirla, incluida la de AutoFirma.
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

    /// La URL que entrega el escritorio, con sus pares y su credencial.
    const A_LAUNCH: &str =
        "afirma://websocket?ports=51000,51001&v=4&idsession=8jAkPZfRw2mQxN4TbYuL";

    fn invoked_with_the_url(url: &str) -> Invocation {
        Invocation {
            command_line: vec!["rfirma".to_owned(), url.to_owned()],
            folder: PathBuf::from("/"),
        }
    }

    /// **ID-235.** La URL de la sede no es una ruta: ni se abre, ni se avisa de
    /// que no se pudo abrir. Antes, `/casa/afirma://websocket?…` se buscaba en
    /// el disco y la ventana anunciaba un fichero que no era un PDF.
    #[test]
    fn a_site_url_is_not_treated_as_a_file_path() {
        let invocation = invoked_with_the_url(A_LAUNCH);

        assert_eq!(invocation.site_launch(), Some(A_LAUNCH));
        assert_eq!(invoked_document(&invocation, &OpenedDocuments::new()), None);
    }

    /// **TD-65.** Lo que se prueba del esquema es que la URL sobrevive **entera**
    /// el camino de la instancia única: el complemento entrega la línea de
    /// órdenes tal cual, y de ahí sale la misma cadena con su `?` y sus pares.
    #[test]
    fn the_whole_url_survives_the_single_instance_path() {
        // La línea de órdenes tal y como la entrega el complemento en la
        // segunda invocación, con la carpeta de trabajo del proceso que la
        // atiende, que es otra.
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

    /// El esquema no distingue mayúsculas: quien entrega la cadena es el
    /// escritorio, y el ejecutable no es un argumento.
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

    /// **ID-236.** Un argumento que no es UTF-8 haría entrar en pánico al
    /// `std::env::args()` del complemento, así que la línea de órdenes se deja
    /// legible antes de que él mire.
    #[test]
    fn an_argument_that_is_not_utf8_is_made_readable_before_the_plugin_reads_it() {
        use std::os::unix::ffi::OsStringExt as _;

        let unreadable = OsString::from_vec(vec![b'/', 0xff, b'.', b'p', b'd', b'f']);

        assert_eq!(
            arguments_before_the_single_instance(vec![OsString::from("rfirma"), unreadable]),
            Arguments::RerunWith(vec!["rfirma".to_owned(), "/\u{fffd}.pdf".to_owned()])
        );
    }

    /// Y una línea de órdenes normal —la URL de la sede lo es siempre— no
    /// vuelve a arrancar nada.
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
