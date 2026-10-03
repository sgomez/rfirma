//! Gestión de la línea de órdenes en el arranque e invocación desde el escritorio (ADR-0010, ADR-0015).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::desktop::domain::command_line::{Command, WindowIntent, SEE_SIGNATURES};
use crate::documents::domain::dropped::invoked_paths;
use crate::site::domain::protocol::{AfirmaUrl, IMPLEMENTED_AUTOFIRMA_VERSION};

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

    fn foreign_launch(&self) -> Option<&str> {
        self.command_line
            .iter()
            .skip(1)
            .map(String::as_str)
            .find(|argument| has_a_foreign_scheme(argument))
    }
}

fn has_a_foreign_scheme(argument: &str) -> bool {
    argument.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.eq_ignore_ascii_case("file")
            && scheme.starts_with(|first: char| first.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
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

pub use crate::desktop::domain::command_line::{HELP_FLAGS, VERSION_FLAGS};

/// Texto informativo mostrado en la ayuda por consola.
pub const HELP: &str = "\
rfirma — firma electrónica con certificado, compatible con AutoFirma.

Uso:
  rfirma [documento…]
  rfirma «afirma://…»
  rfirma <orden> [parámetros…]
  rfirma <orden> --help
  rfirma --help
  rfirma --version

Argumentos:
  documento           Ruta de un PDF: se abre en la ventana, listo para firmar.
                      Lo que no sea un PDF abre la ventana igual y lo dice.
                      También vale como file://…
  afirma://…          La llamada de una sede electrónica. La entrega el
                      navegador a través del manejador del esquema; a mano,
                      sirve para probar. Una URL de cualquier otro esquema
                      no abre nada. Si la hay, gana a cualquier orden.

Opciones:
  -h, --help          Muestra esta ayuda y termina.
  --version           Muestra la versión de rFirma y la de AutoFirma de la que
                      salen los validadores.

Órdenes, las de AutoFirma, sin distinguir mayúsculas y siempre como primer
argumento. Se atienden en la terminal, sin unirse a la ventana de rFirma que
esté abierta, y el proceso termina con ellas:
  sign                Firma un fichero.
  cosign              Añade una firma a un fichero ya firmado.
  listaliases         Lista los certificados de los almacenes.
  verify              Valida las firmas de un fichero.

Parámetros de las órdenes (rfirma <orden> --help da la sintaxis de cada una):
  -i <fichero>        Fichero de entrada.
  -o <fichero>        Fichero de salida, que se sobrescribe si existe.
                      Obligatorio salvo con --xml.
  --format <formato>  auto (por omisión), pades, cades o xades.
  --store <almacén>   Busca solo en ese almacén; sin él, en todos.
  --alias <alias>     Firma con ese certificado, sin preguntar.
  --filter <filtro>   Firma con el único certificado que cumple el filtro, o
                      acota la lista de --certgui o --certtui.
  --certgui           Elige certificado y PIN en la ventana de sede.
  --certtui           Elige certificado en la terminal. Propio de rFirma.
  --password-fd <N>   Lee el PIN del descriptor N, abierto por quien llama.
                      Propio de rFirma.
  --algorithm <alg>   sha512 (por omisión), sha384 o sha256.
  --config <texto>    Propiedades clave=valor de la firma, una por línea, las
                      mismas que se aceptan de una sede.
  --xml               Responde en XML por la salida estándar.
  --gui               Entrega el fichero de -i a la ventana de rFirma, sin
                      firmar ni verificar.
  --help              Muestra la sintaxis de la orden.

Salida de las órdenes:
  El código de salida es 0 si la orden termina bien y distinto de 0 si falla.
  Por la salida estándar solo sale lo que se consume: la sintaxis de --help y
  el XML de --xml. Los mensajes y los registros van a la salida de errores.

Desviaciones de la línea de órdenes de AutoFirma:
  --password          Se rechaza: la contraseña en la línea de órdenes la ve
                      cualquier usuario del equipo y queda en el historial. El
                      PIN se pide en la terminal, se lee de --password-fd o se
                      escribe en la ventana con --certgui.
  countersign, batchsign
                      No existen.
  --preurl, --posturl, --hformat, --halgorithm, -r, --operation
                      No existen.
  --algorithm sha1    Se rechaza.
  --store             Un almacén que AutoFirma no reconoce se rechaza.
  --certgui, --certtui
                      No listan certificados caducados ni cambian de almacén.
  Salida estándar     No mezcla los mensajes con lo que se consume, al
                      contrario que AutoFirma.

Ejemplo: firmar con el PIN guardado en el llavero del escritorio, sin que pase
por la línea de órdenes ni por el historial:
  rfirma sign -i contrato.pdf -o contrato-firmado.pdf --alias mi-certificado \\
      --password-fd 3 3< <(secret-tool lookup service rfirma)

Ejemplo en el flatpak, con un fichero fuera de la carpeta de documentos:
  flatpak run --file-forwarding me.sgomez.rfirma sign -i @@ <fichero> @@ -o <salida>

Lo que rFirma atiende de una sede (protocolo 4, sobre wss:// en 127.0.0.1):
  websocket           Abre el canal en uno de los puertos que sortea la sede.
  echo                Comprobación de vida.
  selectcert          Elegir certificado, consentido por la persona.
  sign                Firma PAdES de un PDF.
  cosign              Cofirma PAdES de un PDF.
  countersign, save y signandsave se rechazan con su código del catálogo.
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

/// Si los argumentos de ejecución piden la versión.
pub fn version_was_asked_for<I, S>(arguments: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments
        .into_iter()
        .skip(1)
        .any(|argument| VERSION_FLAGS.contains(&argument.as_ref()))
}

/// La versión de rFirma y, en una segunda línea, la de AutoFirma de la que salen los validadores.
pub fn version_text(rfirma_version: &str) -> String {
    format!("rfirma {rfirma_version}\nAutoFirma {IMPLEMENTED_AUTOFIRMA_VERSION}")
}

/// Lo que se imprime en lugar de arrancar, si los argumentos piden la ayuda o la versión.
pub fn informative_text(arguments: &[String], rfirma_version: &str) -> Option<String> {
    if help_was_asked_for(arguments) {
        return Some(HELP.to_owned());
    }
    version_was_asked_for(arguments).then(|| version_text(rfirma_version))
}

/// Las rutas que la invocación trae para la ventana principal: nada si es una llamada de sede.
pub fn invoked_documents(invocation: &Invocation) -> Option<Vec<PathBuf>> {
    if invocation.site_launch().is_some() {
        return None;
    }
    let paths = invoked_paths(&invocation.command_line, &invocation.folder);
    (!paths.is_empty()).then_some(paths)
}

/// Para qué trae la invocación sus documentos: verlos firmados, si la entregó `verify -gui`.
pub fn invoked_intent(invocation: &Invocation) -> WindowIntent {
    if invocation
        .command_line
        .iter()
        .skip(1)
        .any(|argument| argument == SEE_SIGNATURES)
    {
        WindowIntent::SeeItsSignatures
    } else {
        WindowIntent::OpenTheDocument
    }
}

/// Los argumentos con los que el proceso de terminal lanza el de escritorio para entregarle un fichero.
pub fn arguments_for_the_desktop(file: &Path, intent: WindowIntent) -> Vec<OsString> {
    match intent {
        WindowIntent::OpenTheDocument => vec![file.into()],
        WindowIntent::SeeItsSignatures => vec![SEE_SIGNATURES.into(), file.into()],
    }
}

/// Destino de una segunda invocación recibida con la aplicación ya en marcha.
#[derive(Debug, PartialEq, Eq)]
pub enum SecondInvocation {
    /// Se ignora la segunda invocación.
    NothingHappens,
    /// Sustituye el documento activo por lo que traen estas rutas, con su intención.
    ReplacesWhatWasThere(Vec<PathBuf>, WindowIntent),
}

/// Determina la acción a tomar ante una segunda invocación del proceso, que solo puede ser
/// del escritorio (ADR-0024): una URL `afirma://` por el bus la manda un binario viejo y no
/// trae documentos, así que cae sola en `NothingHappens`.
pub fn second_invocation(invocation: &Invocation, signing_is_live: bool) -> SecondInvocation {
    if signing_is_live {
        return SecondInvocation::NothingHappens;
    }
    match invoked_documents(invocation) {
        Some(paths) => SecondInvocation::ReplacesWhatWasThere(paths, invoked_intent(invocation)),
        None => SecondInvocation::NothingHappens,
    }
}

/// Rol de este proceso según su línea de órdenes (ADR-0024, ADR-0041).
#[derive(Debug, PartialEq, Eq)]
pub enum Role {
    /// El escritorio, con la invocación completa.
    Desktop(Invocation),
    /// La sede, con la URL `afirma://` entera.
    Site(String),
    /// La terminal, con los argumentos que siguen al ejecutable, empezando por la orden.
    Terminal(Vec<String>),
    /// Ninguno: una URL de otro esquema no abre ventana, como en el original.
    Foreign(String),
}

/// Decide el rol de este proceso. Una URL `afirma://`, si la hay, gana siempre; cualquier
/// documento que la acompañe se descarta.
pub fn role_of(invocation: Invocation) -> Role {
    if let Some(url) = invocation.site_launch() {
        return Role::Site(url.to_owned());
    }
    let arguments = invocation.command_line.get(1..).unwrap_or_default();
    if arguments
        .first()
        .is_some_and(|first| Command::named(first).is_some())
    {
        return Role::Terminal(arguments.to_vec());
    }
    match invocation.foreign_launch() {
        Some(url) => Role::Foreign(url.to_owned()),
        None => Role::Desktop(invocation),
    }
}

/// Lo que hace un proceso con las URL que el sistema le entrega fuera de su línea de órdenes.
#[derive(Debug, PartialEq, Eq)]
pub struct DeliveredUrls {
    /// Las URL `afirma://`, cada una para un proceso de sede propio.
    pub site_launches: Vec<String>,
    /// Si este proceso sigue: no, cuando solo arrancó para entregarlas.
    pub this_process_goes_on: bool,
}

/// Reparte en procesos de sede las URL entregadas, igual que si llegaran por la línea de
/// órdenes (ADR-0024).
pub fn delivered_urls<I>(urls: I, already_serving: bool) -> DeliveredUrls
where
    I: IntoIterator<Item = String>,
{
    let site_launches: Vec<String> = urls
        .into_iter()
        .filter(|url| AfirmaUrl::is_a_protocol_url(url))
        .collect();
    DeliveredUrls {
        this_process_goes_on: already_serving || site_launches.is_empty(),
        site_launches,
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
