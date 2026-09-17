//! Gestión de la línea de órdenes en el arranque e invocación desde el escritorio (ADR-0010, ADR-0015).

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::documents::domain::dropped::invoked_paths;
use crate::site::domain::protocol::AfirmaUrl;

/// Invocación recibida con sus argumentos y carpeta de trabajo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invocation {
    /// Argumentos de la línea de órdenes.
    pub command_line: Vec<String>,
    /// Directorio de trabajo en el momento de la invocación.
    pub folder: PathBuf,
}

impl Invocation {
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

/// Las rutas que la invocación trae para la ventana principal: nada si es una llamada de sede.
pub fn invoked_documents(invocation: &Invocation) -> Option<Vec<PathBuf>> {
    if invocation.site_launch().is_some() {
        return None;
    }
    let paths = invoked_paths(&invocation.command_line, &invocation.folder);
    (!paths.is_empty()).then_some(paths)
}

/// Destino de una segunda invocación recibida con la aplicación ya en marcha.
#[derive(Debug, PartialEq, Eq)]
pub enum SecondInvocation {
    /// Se ignora la segunda invocación.
    NothingHappens,
    /// Sustituye el documento activo por lo que traen estas rutas.
    ReplacesWhatWasThere(Vec<PathBuf>),
}

/// Determina la acción a tomar ante una segunda invocación del proceso, que solo puede ser
/// del escritorio (ADR-0024): una URL `afirma://` por el bus la manda un binario viejo y no
/// trae documentos, así que cae sola en `NothingHappens`.
pub fn second_invocation(invocation: &Invocation, signing_is_live: bool) -> SecondInvocation {
    if signing_is_live {
        return SecondInvocation::NothingHappens;
    }
    match invoked_documents(invocation) {
        Some(paths) => SecondInvocation::ReplacesWhatWasThere(paths),
        None => SecondInvocation::NothingHappens,
    }
}

/// Rol de este proceso según su línea de órdenes: escritorio o sede (ADR-0024).
#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    /// El escritorio, con la invocación completa.
    Desktop(Invocation),
    /// La sede, con la URL `afirma://` entera.
    Site(String),
}

/// Decide el rol de este proceso. Una URL `afirma://`, si la hay, gana siempre; cualquier
/// documento que la acompañe se descarta.
pub fn role_of(invocation: Invocation) -> Role {
    match invocation.site_launch() {
        Some(url) => Role::Site(url.to_owned()),
        None => Role::Desktop(invocation),
    }
}

impl Role {
    /// La línea que narra el documento descartado cuando la invocación traía uno junto a la
    /// URL de sede.
    pub fn said(invocation: &Invocation) -> Vec<String> {
        let Some(url) = invocation.site_launch() else {
            return Vec::new();
        };
        let rest: Vec<String> = invocation
            .command_line
            .iter()
            .filter(|argument| argument.as_str() != url)
            .cloned()
            .collect();
        if invoked_paths(&rest, &invocation.folder).is_empty() {
            return Vec::new();
        }
        vec![
            "rfirma: la llamada de sede tiene prioridad; se descarta el documento de la línea \
             de órdenes"
                .to_owned(),
        ]
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
        crate::lock(&self.0).take()
    }
}

#[cfg(test)]
mod tests;
