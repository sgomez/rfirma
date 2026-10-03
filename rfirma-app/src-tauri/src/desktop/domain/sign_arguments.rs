//! Los argumentos de `sign` y `cosign` ya analizados y lo que se rechaza de ellos, sin abrir ningún almacén.

use std::fmt;

use super::command_line::{as_documented, documented};

const WITH_VALUE: [&str; 9] = [
    "-i",
    "-o",
    "-format",
    "-algorithm",
    "-alias",
    "-filter",
    "-config",
    "-store",
    "-password-fd",
];

const SWITCHES: [&str; 4] = ["-certtui", "-certgui", "-xml", "-gui"];

/// Si es un parámetro de `sign` o `cosign`, en la forma `-opción`.
pub fn is_an_option(argument: &str) -> bool {
    WITH_VALUE.contains(&argument) || SWITCHES.contains(&argument)
}

/// El formato de firma que se pide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Auto,
    Pades,
    Cades,
    Xades,
}

/// El algoritmo de huella que se pide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Algorithm {
    #[default]
    Sha512,
    Sha384,
    Sha256,
}

/// Cómo se elige el certificado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Alias(String),
    Filter(String),
    Terminal { filter: Option<String> },
    Window { filter: Option<String> },
}

/// Los argumentos de una orden de firma que pasan todas las comprobaciones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignArguments {
    pub input: String,
    pub output: Option<String>,
    pub format: Format,
    pub algorithm: Algorithm,
    pub selection: Option<Selection>,
    pub store: Option<String>,
    pub config: Option<String>,
    pub password_fd: Option<u32>,
    pub xml: bool,
    pub hand_to_window: bool,
}

/// Por qué los argumentos de una orden de firma no se aceptan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArgumentsRefusal {
    UnknownArgument(String),
    MissingValue(&'static str),
    MissingInput,
    MissingOutput,
    UnsupportedFormat(String),
    UnknownFormat(String),
    Sha1Refused,
    UnknownAlgorithm(String),
    InvalidDescriptor(String),
    NoCertificateSelection,
    ConflictingSelection,
}

impl fmt::Display for ArgumentsRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(argument) => {
                write!(formatter, "el argumento «{}» no se reconoce", as_documented(argument))
            }
            Self::MissingValue(parameter) => {
                write!(formatter, "el parámetro {} necesita un valor", documented(parameter))
            }
            Self::MissingInput => write!(formatter, "falta -i <fichero>"),
            Self::MissingOutput => write!(formatter, "falta -o <fichero>, salvo con --xml"),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "el formato «{format}» no está soportado")
            }
            Self::UnknownFormat(format) => write!(
                formatter,
                "el formato «{format}» no se reconoce: auto, pades, cades o xades"
            ),
            Self::Sha1Refused => write!(
                formatter,
                "sha1 no se acepta por ser débil: usa sha512, sha384 o sha256"
            ),
            Self::UnknownAlgorithm(algorithm) => write!(
                formatter,
                "el algoritmo «{algorithm}» no se reconoce: sha512, sha384 o sha256"
            ),
            Self::InvalidDescriptor(value) => write!(
                formatter,
                "--password-fd necesita el número de un descriptor, no «{value}»"
            ),
            Self::NoCertificateSelection => write!(
                formatter,
                "falta elegir el certificado: usa uno de --alias, --certgui o --certtui, o --filter"
            ),
            Self::ConflictingSelection => write!(
                formatter,
                "--alias, --certgui y --certtui se excluyen entre sí, y --alias tampoco admite --filter"
            ),
        }
    }
}

#[derive(Default)]
struct Collected {
    values: Vec<(&'static str, String)>,
    switches: Vec<&'static str>,
}

impl Collected {
    fn value(&self, parameter: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(name, _)| *name == parameter)
            .map(|(_, value)| value.as_str())
    }

    fn has(&self, switch: &str) -> bool {
        self.switches.contains(&switch)
    }
}

fn collect(arguments: &[String]) -> Result<Collected, ArgumentsRefusal> {
    let mut collected = Collected::default();
    let mut remaining = arguments.iter();
    while let Some(argument) = remaining.next() {
        if let Some(parameter) = WITH_VALUE.into_iter().find(|name| name == argument) {
            let value = remaining
                .next()
                .ok_or(ArgumentsRefusal::MissingValue(parameter))?;
            collected.values.push((parameter, value.clone()));
        } else if let Some(switch) = SWITCHES.into_iter().find(|name| name == argument) {
            collected.switches.push(switch);
        } else {
            return Err(ArgumentsRefusal::UnknownArgument(argument.clone()));
        }
    }
    Ok(collected)
}

fn format_of(collected: &Collected) -> Result<Format, ArgumentsRefusal> {
    let Some(name) = collected.value("-format") else {
        return Ok(Format::default());
    };
    match name.to_ascii_lowercase().as_str() {
        "auto" => Ok(Format::Auto),
        "pades" => Ok(Format::Pades),
        "cades" => Ok(Format::Cades),
        "xades" => Ok(Format::Xades),
        "facturae" | "ooxml" | "odf" => Err(ArgumentsRefusal::UnsupportedFormat(name.to_owned())),
        _ => Err(ArgumentsRefusal::UnknownFormat(name.to_owned())),
    }
}

fn algorithm_of(collected: &Collected) -> Result<Algorithm, ArgumentsRefusal> {
    let Some(name) = collected.value("-algorithm") else {
        return Ok(Algorithm::default());
    };
    match name.to_ascii_lowercase().as_str() {
        "sha512" => Ok(Algorithm::Sha512),
        "sha384" => Ok(Algorithm::Sha384),
        "sha256" => Ok(Algorithm::Sha256),
        "sha1" => Err(ArgumentsRefusal::Sha1Refused),
        _ => Err(ArgumentsRefusal::UnknownAlgorithm(name.to_owned())),
    }
}

fn selection_of(collected: &Collected) -> Result<Selection, ArgumentsRefusal> {
    let alias = collected.value("-alias");
    let filter = collected.value("-filter").map(str::to_owned);
    let graphical = collected.has("-certgui");
    let terminal = collected.has("-certtui");
    let chosen = usize::from(alias.is_some()) + usize::from(graphical) + usize::from(terminal);
    if chosen > 1 || (alias.is_some() && filter.is_some()) {
        return Err(ArgumentsRefusal::ConflictingSelection);
    }
    match (alias, terminal, filter) {
        (Some(alias), _, _) => Ok(Selection::Alias(alias.to_owned())),
        (None, false, filter) if graphical => Ok(Selection::Window { filter }),
        (None, true, filter) => Ok(Selection::Terminal { filter }),
        (None, false, Some(filter)) => Ok(Selection::Filter(filter)),
        (None, false, None) => Err(ArgumentsRefusal::NoCertificateSelection),
    }
}

fn descriptor_of(collected: &Collected) -> Result<Option<u32>, ArgumentsRefusal> {
    collected
        .value("-password-fd")
        .map(|value| {
            value
                .parse()
                .map_err(|_| ArgumentsRefusal::InvalidDescriptor(value.to_owned()))
        })
        .transpose()
}

/// Analiza los argumentos que siguen a `sign` o a `cosign` y rechaza lo inválido.
pub fn parse_sign_arguments(arguments: &[String]) -> Result<SignArguments, ArgumentsRefusal> {
    let collected = collect(arguments)?;
    let input = collected
        .value("-i")
        .ok_or(ArgumentsRefusal::MissingInput)?
        .to_owned();
    let hand_to_window = collected.has("-gui");
    let xml = collected.has("-xml");
    let output = collected.value("-o").map(str::to_owned);
    let format = format_of(&collected)?;
    let algorithm = algorithm_of(&collected)?;
    let password_fd = descriptor_of(&collected)?;
    let selection = if hand_to_window {
        None
    } else {
        if output.is_none() && !xml {
            return Err(ArgumentsRefusal::MissingOutput);
        }
        Some(selection_of(&collected)?)
    };
    Ok(SignArguments {
        input,
        output,
        format,
        algorithm,
        selection,
        store: collected.value("-store").map(str::to_owned),
        config: collected.value("-config").map(str::to_owned),
        password_fd,
        xml,
        hand_to_window,
    })
}

#[cfg(test)]
mod tests;
