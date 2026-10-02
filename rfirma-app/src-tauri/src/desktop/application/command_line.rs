//! El caso de uso de la línea de órdenes: atiende una orden y dice qué sale por cada flujo, sin escribir en ninguno.

use std::path::Path;

use crate::desktop::application::store_scope::{within_the_scope, ScopeFailure};
use crate::desktop::domain::command_line::{
    command_of, file_for_the_window, is_a_help_flag, parameter_left_out, Command, Refusal,
};
use crate::desktop::domain::sign_arguments::parse_sign_arguments;
use crate::desktop::domain::store_scope::scope_named_by;
use crate::desktop::ports::{
    CertificateStores, CommandLineFiles, DesktopHandover, SignatureVerifier, Terminal,
};
use crate::identity::domain::certificate::TokenCertificate;

mod verify;
pub use verify::{format_to_verify, UNKNOWN_FORMAT};

/// El código de salida de una orden que termina bien.
pub const SUCCEEDED: i32 = 0;

/// El código de salida de una orden que falla al atenderse.
pub const FAILED: i32 = 1;

/// El código de salida de una línea de órdenes que no se atiende tal como llega.
pub const REFUSED: i32 = 2;

/// Lo que la línea de órdenes alcanza del mundo.
pub struct CommandLinePorts<'a> {
    /// Los almacenes de certificados.
    pub stores: &'a dyn CertificateStores,
    /// La terminal desde la que se lanza la orden.
    pub terminal: &'a dyn Terminal,
    /// El proceso de escritorio que recibe los ficheros de `-gui`.
    pub desktop: &'a dyn DesktopHandover,
    /// Los ficheros que nombran los argumentos.
    pub files: &'a dyn CommandLineFiles,
    /// El validador del original.
    pub verifier: &'a dyn SignatureVerifier,
}

/// Lo que una orden deja al terminar: código de salida, bytes de stdout y líneas de stderr.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<String>,
}

impl Outcome {
    fn refused(refusal: &Refusal) -> Self {
        Self {
            exit_code: REFUSED,
            stdout: Vec::new(),
            stderr: vec![
                format!("rfirma: {refusal}"),
                "rfirma: «rfirma --help» describe las órdenes y sus parámetros".to_owned(),
            ],
        }
    }

    fn failed(line: String) -> Self {
        Self {
            exit_code: FAILED,
            stdout: Vec::new(),
            stderr: vec![line],
        }
    }

    fn printed(lines: &[String]) -> Self {
        Self {
            exit_code: SUCCEEDED,
            stdout: lines
                .iter()
                .map(|line| format!("{line}\n"))
                .collect::<String>()
                .into_bytes(),
            stderr: Vec::new(),
        }
    }

    fn not_yet_available(what: &str) -> Self {
        Self::failed(format!(
            "rfirma: {what} todavía no está disponible en esta versión"
        ))
    }

    fn aliases_of(certificates: &[TokenCertificate]) -> Self {
        let stdout = certificates
            .iter()
            .map(|certificate| format!("{}\n", certificate.reference().label()))
            .collect::<String>()
            .into_bytes();
        let stderr = if certificates.is_empty() {
            vec!["rfirma: no hay ningún certificado en los almacenes".to_owned()]
        } else {
            Vec::new()
        };
        Self {
            exit_code: SUCCEEDED,
            stdout,
            stderr,
        }
    }
}

/// Atiende los argumentos que siguen al ejecutable, empezando por la orden.
pub fn attend(arguments: &[String], ports: &CommandLinePorts) -> Outcome {
    let command = match command_of(arguments) {
        Ok(command) => command,
        Err(refusal) => return Outcome::refused(&refusal),
    };
    if arguments
        .iter()
        .skip(1)
        .any(|argument| is_a_help_flag(argument))
    {
        return Outcome {
            exit_code: SUCCEEDED,
            stdout: command.syntax().unwrap_or_default().as_bytes().to_vec(),
            stderr: Vec::new(),
        };
    }
    if let Some(refusal) = parameter_left_out(arguments) {
        return Outcome::refused(&refusal);
    }
    if matches!(command, Command::Sign | Command::Cosign) {
        if let Err(refusal) = parse_sign_arguments(&arguments[1..]) {
            return Outcome::refused(&Refusal::InvalidArguments(refusal));
        }
    }
    match file_for_the_window(command, arguments) {
        Err(refusal) => Outcome::refused(&refusal),
        Ok(Some(file)) => hand_over_to_the_window(ports.desktop, file),
        Ok(None) => carried_out(command, arguments, ports),
    }
}

fn hand_over_to_the_window(desktop: &dyn DesktopHandover, file: &str) -> Outcome {
    match desktop.hand_over(Path::new(file)) {
        Ok(()) => Outcome::default(),
        Err(reason) => Outcome::failed(format!("rfirma: no se puede abrir la ventana ({reason})")),
    }
}

fn carried_out(command: Command, arguments: &[String], ports: &CommandLinePorts) -> Outcome {
    match command {
        Command::ListAliases => list_aliases(arguments, ports.stores),
        Command::Verify => verify::verify(arguments, ports),
        _ => Outcome::not_yet_available(&format!("la orden «{}»", command.name())),
    }
}

fn list_aliases(arguments: &[String], stores: &dyn CertificateStores) -> Outcome {
    let scope = match scope_named_by(arguments) {
        Ok(scope) => scope,
        Err(refusal) => return Outcome::refused(&Refusal::InvalidStore(refusal)),
    };
    match within_the_scope(&scope, stores) {
        Ok(certificates) => Outcome::aliases_of(&certificates),
        Err(ScopeFailure::Token(error)) => Outcome::failed(format!(
            "rfirma: no se ha podido abrir ningún almacén de certificados ({})",
            error.detail()
        )),
        Err(ScopeFailure::ModuleNotDiscovered(library)) => Outcome::failed(format!(
            "rfirma: «{library}» no es un módulo PKCS#11 de los que rfirma ha descubierto, \
             y no carga ninguno solo porque lo nombre la orden"
        )),
    }
}

#[cfg(test)]
mod tests;
