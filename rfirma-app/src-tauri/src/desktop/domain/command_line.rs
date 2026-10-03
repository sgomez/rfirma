//! Las órdenes de la línea de órdenes de AutoFirma que reconoce rFirma y lo que rechaza de ellas, no su ejecución.

use std::fmt;

use serde::Serialize;

use super::sign_arguments::{self, ArgumentsRefusal};
use super::store_scope::StoreRefusal;

/// Formas aceptadas del parámetro de ayuda.
pub const HELP_FLAGS: [&str; 3] = ["--help", "-help", "-h"];

/// Formas aceptadas del parámetro de la versión.
pub const VERSION_FLAGS: [&str; 2] = ["--version", "-version"];

/// El parámetro de la contraseña del original, que nunca se acepta en argv.
pub const PASSWORD: &str = "-password";

/// La alternativa a `-password`: un descriptor que abre quien llama.
pub const PASSWORD_FD: &str = "-password-fd";

/// El parámetro que pide la ventana de rFirma en lugar de firmar o verificar.
pub const GUI: &str = "-gui";

/// El parámetro del fichero de entrada.
pub const INPUT: &str = "-i";

/// El parámetro que acota los almacenes donde se buscan los certificados.
pub const STORE: &str = "-store";

/// El parámetro que pide el resultado en XML.
pub const XML: &str = "-xml";

/// Las dos formas del parámetro que añade a `verify` la ficha de cada firma.
pub const VERBOSE: [&str; 2] = ["-v", "-verbose"];

/// Parámetros del original que rFirma no atiende.
pub const PARAMETERS_LEFT_OUT: [&str; 5] = ["-preurl", "-posturl", "-hformat", "-halgorithm", "-r"];

/// Una orden del original, reconocida sin distinguir mayúsculas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Sign,
    Cosign,
    ListAliases,
    Verify,
    CounterSign,
    BatchSign,
}

const COMMANDS: [Command; 6] = [
    Command::Sign,
    Command::Cosign,
    Command::ListAliases,
    Command::Verify,
    Command::CounterSign,
    Command::BatchSign,
];

impl Command {
    /// La orden que nombra este argumento, si nombra alguna.
    pub fn named(argument: &str) -> Option<Self> {
        COMMANDS
            .into_iter()
            .find(|command| command.name().eq_ignore_ascii_case(argument))
    }

    /// El nombre de la orden tal como lo escribe el original.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sign => "sign",
            Self::Cosign => "cosign",
            Self::ListAliases => "listaliases",
            Self::Verify => "verify",
            Self::CounterSign => "countersign",
            Self::BatchSign => "batchsign",
        }
    }

    /// La sintaxis de la orden, o nada si rFirma no la atiende.
    pub fn syntax(self) -> Option<&'static str> {
        match self {
            Self::Sign => Some(SIGN_SYNTAX),
            Self::Cosign => Some(COSIGN_SYNTAX),
            Self::ListAliases => Some(LIST_ALIASES_SYNTAX),
            Self::Verify => Some(VERIFY_SYNTAX),
            Self::CounterSign | Self::BatchSign => None,
        }
    }
}

/// El parámetro con la forma que documenta rFirma: `--opción` si tiene más de una letra.
pub fn documented(parameter: &str) -> String {
    match parameter.strip_prefix('-') {
        Some(name) if name.len() > 1 => format!("--{name}"),
        _ => parameter.to_owned(),
    }
}

/// El argumento en la forma documentada si es una opción conocida, o tal como llegó.
pub fn as_documented(argument: &str) -> String {
    match argument.strip_prefix('-') {
        Some(name) if is_a_known_option(name) => documented(argument),
        _ => argument.to_owned(),
    }
}

/// Los argumentos con cada `--opción` conocida en la forma `-opción` que usa el resto del código.
pub fn normalised(arguments: &[String]) -> Vec<String> {
    arguments
        .iter()
        .map(|argument| match argument.strip_prefix("--") {
            Some(name) if is_a_known_option(name) => format!("-{name}"),
            _ => argument.clone(),
        })
        .collect()
}

fn is_a_known_option(name: &str) -> bool {
    if name.len() < 2 {
        return false;
    }
    let internal = format!("-{name}");
    sign_arguments::is_an_option(&internal)
        || internal.eq_ignore_ascii_case(PASSWORD)
        || PARAMETERS_LEFT_OUT.contains(&internal.as_str())
        || internal == "-help"
        || VERBOSE.contains(&internal.as_str())
}

/// El primer argumento que pide el detalle de `verify` en una orden que no es `verify`.
pub fn verbose_outside_verify(command: Command, arguments: &[String]) -> Option<Refusal> {
    if command == Command::Verify {
        return None;
    }
    arguments
        .iter()
        .skip(1)
        .find(|argument| levels_asked_by(argument) > 0)
        .map(|argument| Refusal::UnknownArgument(argument.clone()))
}

/// Cuántas veces piden los argumentos, ya normalizados, el detalle de `verify`.
pub fn verbosity(arguments: &[String]) -> usize {
    arguments
        .iter()
        .map(|argument| levels_asked_by(argument))
        .sum()
}

fn levels_asked_by(argument: &str) -> usize {
    if VERBOSE.contains(&argument) {
        return 1;
    }
    match argument.strip_prefix('-') {
        Some(letters) if letters.len() > 1 && letters.bytes().all(|letter| letter == b'v') => {
            letters.len()
        }
        _ => 0,
    }
}

/// Si el argumento pide la ayuda.
pub fn is_a_help_flag(argument: &str) -> bool {
    HELP_FLAGS.contains(&argument)
}

/// Por qué una línea de órdenes no se atiende tal como llega.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Falta la orden.
    NoCommand,
    /// El primer argumento no es una orden.
    UnknownCommand(String),
    /// La contraseña viene en argv.
    PasswordInArgv,
    /// Una orden del original que rFirma no atiende.
    CommandLeftOut(Command),
    /// Un parámetro del original que rFirma no atiende.
    ParameterLeftOut(&'static str),
    /// Un argumento que la orden no tiene.
    UnknownArgument(String),
    /// Los argumentos de `sign` o `cosign` no son válidos.
    InvalidArguments(ArgumentsRefusal),
    /// `-gui` sin el fichero de `-i`.
    GuiWithoutInput,
    /// El valor de `-store` no se atiende.
    InvalidStore(StoreRefusal),
    /// `listaliases` con `-password-fd`: listar no pide PIN.
    PasswordForListing,
    /// Falta un parámetro que la orden necesita, o su valor.
    MissingParameter(&'static str),
}

impl fmt::Display for Refusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCommand => write!(formatter, "falta la orden"),
            Self::UnknownCommand(argument) => {
                write!(formatter, "«{argument}» no es una orden de rfirma")
            }
            Self::PasswordInArgv => write!(
                formatter,
                "{} no se acepta: la contraseña en la línea de órdenes la ve \
                 cualquier usuario del equipo; usa {} <N>, la terminal o --certgui",
                documented(PASSWORD),
                documented(PASSWORD_FD)
            ),
            Self::CommandLeftOut(command) => write!(
                formatter,
                "la orden «{}» de AutoFirma no está disponible en rfirma",
                command.name()
            ),
            Self::ParameterLeftOut(parameter) => write!(
                formatter,
                "el parámetro {} de AutoFirma no está disponible en rfirma",
                documented(parameter)
            ),
            Self::UnknownArgument(argument) => write!(
                formatter,
                "el argumento «{}» no se reconoce",
                as_documented(argument)
            ),
            Self::InvalidArguments(refusal) => write!(formatter, "{refusal}"),
            Self::InvalidStore(refusal) => write!(formatter, "{refusal}"),
            Self::PasswordForListing => write!(
                formatter,
                "{} no se acepta en listaliases: listar no pide PIN",
                documented(PASSWORD_FD)
            ),
            Self::MissingParameter(parameter) => {
                write!(
                    formatter,
                    "falta el parámetro {} con su valor",
                    documented(parameter)
                )
            }
            Self::GuiWithoutInput => {
                write!(
                    formatter,
                    "{} necesita el fichero que se abre, con {INPUT} <fichero>",
                    documented(GUI)
                )
            }
        }
    }
}

/// La orden de unos argumentos que empiezan por ella, o lo primero que se rechaza de ellos.
pub fn command_of(arguments: &[String]) -> Result<Command, Refusal> {
    if arguments
        .iter()
        .any(|argument| argument.eq_ignore_ascii_case(PASSWORD))
    {
        return Err(Refusal::PasswordInArgv);
    }
    let Some(first) = arguments.first() else {
        return Err(Refusal::NoCommand);
    };
    let command = Command::named(first).ok_or_else(|| Refusal::UnknownCommand(first.clone()))?;
    if command.syntax().is_none() {
        return Err(Refusal::CommandLeftOut(command));
    }
    Ok(command)
}

/// El primer parámetro de los argumentos que rFirma no atiende.
pub fn parameter_left_out(arguments: &[String]) -> Option<Refusal> {
    arguments.iter().find_map(|argument| {
        PARAMETERS_LEFT_OUT
            .into_iter()
            .find(|parameter| *parameter == argument)
            .map(Refusal::ParameterLeftOut)
    })
}

/// Para qué recibe la ventana el fichero que le entrega `-gui`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowIntent {
    /// Abrirlo, como al soltarlo: lo pide `sign`.
    OpenTheDocument,
    /// Abrirlo para ver sus firmas: lo pide `verify`.
    SeeItsSignatures,
}

/// El parámetro con el que el proceso de escritorio recibe la intención de ver las firmas.
pub const SEE_SIGNATURES: &str = "--see-signatures";

/// Lo que `-gui` entrega a la ventana: el fichero de `-i` y para qué.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowHandover<'a> {
    pub file: &'a str,
    pub intent: WindowIntent,
}

/// Lo que `-gui` entrega a la ventana, o nada si la orden no pide la ventana.
pub fn handover_to_the_window(
    command: Command,
    arguments: &[String],
) -> Result<Option<WindowHandover<'_>>, Refusal> {
    let intent = match command {
        Command::Sign => WindowIntent::OpenTheDocument,
        Command::Verify => WindowIntent::SeeItsSignatures,
        _ => return Ok(None),
    };
    if !arguments.iter().any(|argument| argument == GUI) {
        return Ok(None);
    }
    arguments
        .windows(2)
        .find(|pair| pair[0] == INPUT)
        .map(|pair| {
            Some(WindowHandover {
                file: pair[1].as_str(),
                intent,
            })
        })
        .ok_or(Refusal::GuiWithoutInput)
}

/// El valor que sigue al parámetro en los argumentos, si lo trae.
pub fn value_of<'a>(arguments: &'a [String], parameter: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == parameter)
        .and_then(|at| arguments.get(at + 1))
        .map(String::as_str)
}

const SIGN_SYNTAX: &str = "\
Uso: rfirma sign -i <fichero> (-o <fichero> | --xml)
                 (--alias <alias> | --filter <filtro> | --certgui | --certtui)
                 [--filter <filtro>] [--store <almacén>]
                 [--format auto|pades|cades|xades] [--algorithm sha512|sha384|sha256]
                 [--config <propiedades>] [--password-fd <N>]
     rfirma sign --gui -i <fichero>

Firma el fichero de -i y escribe la firma en -o, que se sobrescribe si existe.
Con --gui, entrega el fichero a la ventana de rFirma y no firma.
";

const COSIGN_SYNTAX: &str = "\
Uso: rfirma cosign -i <fichero> (-o <fichero> | --xml)
                   (--alias <alias> | --filter <filtro> | --certgui | --certtui)
                   [--filter <filtro>] [--store <almacén>]
                   [--format auto|pades|cades|xades] [--algorithm sha512|sha384|sha256]
                   [--config <propiedades>] [--password-fd <N>]

Añade una firma al fichero ya firmado de -i y escribe el resultado en -o, que se
sobrescribe si existe.
";

const LIST_ALIASES_SYNTAX: &str = "\
Uso: rfirma listaliases [--store <almacén>] [--xml]

Lista los certificados de los almacenes, o solo los de --store.
";

const VERIFY_SYNTAX: &str = "\
Uso: rfirma verify -i <fichero> [-v | -vv | --verbose] [--xml]
     rfirma verify --gui -i <fichero>

Valida las firmas del fichero de -i, con la caducidad del certificado del
firmante y sin revocación ni red. Sale con 0 aunque la firma no sea válida,
como AutoFirma. Con --gui, entrega el fichero a la ventana de rFirma.

Con -v, añade el formato y una ficha por firma: firmante, en nombre de quién
firma si es un certificado de representación, emisor y fecha declarada. Con -vv
(o -v -v, --verbose --verbose), añade el número de serie del certificado. Esa
parte no es estable: no la analices con un programa.
";

#[cfg(test)]
mod tests;
