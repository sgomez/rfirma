//! El caso de uso de la línea de órdenes: atiende una orden y dice qué sale por cada flujo, sin escribir en ninguno.

use std::path::Path;

use crate::desktop::application::store_scope::{within_the_scope, ScopeFailure};
use crate::desktop::domain::command_line::{
    command_of, file_for_the_window, is_a_help_flag, parameter_left_out, Command, Refusal,
};
use crate::desktop::domain::sign_arguments::{
    parse_sign_arguments, Format, Selection, SignArguments,
};
use crate::desktop::domain::store_scope::scope_named_by;
use crate::desktop::ports::{
    CertificateStores, CommandLineFiles, CommandLineSigning, DesktopHandover, DocumentSigner,
    SignatureVerifier, Terminal,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::StoreClass;
use crate::signing::domain::bridge::Format as SignatureFormat;

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
    /// Los ficheros que se leen y se escriben.
    pub files: &'a dyn CommandLineFiles,
    /// El validador del original.
    pub verifier: &'a dyn SignatureVerifier,
    /// Quien firma por el camino de la sede.
    pub signer: &'a dyn DocumentSigner,
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
    let signing = if matches!(command, Command::Sign | Command::Cosign) {
        match parse_sign_arguments(&arguments[1..]) {
            Ok(parsed) => Some(parsed),
            Err(refusal) => return Outcome::refused(&Refusal::InvalidArguments(refusal)),
        }
    } else {
        None
    };
    match file_for_the_window(command, arguments) {
        Err(refusal) => Outcome::refused(&refusal),
        Ok(Some(file)) => hand_over_to_the_window(ports.desktop, file),
        Ok(None) => carried_out(command, arguments, signing.as_ref(), ports),
    }
}

fn hand_over_to_the_window(desktop: &dyn DesktopHandover, file: &str) -> Outcome {
    match desktop.hand_over(Path::new(file)) {
        Ok(()) => Outcome::default(),
        Err(reason) => Outcome::failed(format!("rfirma: no se puede abrir la ventana ({reason})")),
    }
}

fn carried_out(
    command: Command,
    arguments: &[String],
    signing: Option<&SignArguments>,
    ports: &CommandLinePorts,
) -> Outcome {
    match (command, signing) {
        (Command::ListAliases, _) => list_aliases(arguments, ports.stores),
        (Command::Verify, _) => verify::verify(arguments, ports),
        (Command::Sign, Some(parsed)) => sign(arguments, parsed, ports),
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
        Err(failure) => failure_of_the_scope(&failure),
    }
}

fn sign(arguments: &[String], parsed: &SignArguments, ports: &CommandLinePorts) -> Outcome {
    let Some(Selection::Alias(alias)) = &parsed.selection else {
        return Outcome::not_yet_available("elegir el certificado sin -alias");
    };
    let Some(output) = &parsed.output else {
        return Outcome::not_yet_available("-xml");
    };
    if let Some(parameter) = not_yet_available_in(parsed) {
        return Outcome::not_yet_available(parameter);
    }
    let certificate = match the_certificate_named(alias, arguments, ports.stores) {
        Ok(certificate) => certificate,
        Err(outcome) => return outcome,
    };
    let input = Path::new(&parsed.input);
    let bytes = match ports.files.read(input) {
        Ok(bytes) => bytes,
        Err(reason) => {
            return Outcome::failed(format!(
                "rfirma: no se puede leer «{}» ({reason})",
                parsed.input
            ))
        }
    };
    let Some(format) = signature_format_of(parsed.format, &bytes) else {
        return Outcome::not_yet_available("la firma CAdES y XAdES");
    };
    let signed = match ports.signer.sign(&CommandLineSigning {
        input,
        certificate: &certificate,
        format,
        algorithm: parsed.algorithm,
    }) {
        Ok(signed) => signed,
        Err(reason) => {
            return Outcome::failed(format!("rfirma: no se ha podido firmar ({reason})"))
        }
    };
    if let Err(reason) = ports.files.write(Path::new(output), &signed) {
        return Outcome::failed(format!(
            "rfirma: no se puede escribir «{output}» ({reason})"
        ));
    }
    ports.signer.remember(&certificate);
    Outcome {
        exit_code: SUCCEEDED,
        stdout: Vec::new(),
        stderr: vec![format!("rfirma: firma guardada en «{output}»")],
    }
}

fn not_yet_available_in(parsed: &SignArguments) -> Option<&'static str> {
    if parsed.xml {
        Some("-xml")
    } else if parsed.config.is_some() {
        Some("-config")
    } else if parsed.password_fd.is_some() {
        Some("-password-fd")
    } else {
        None
    }
}

fn signature_format_of(asked: Format, bytes: &[u8]) -> Option<SignatureFormat> {
    match asked {
        Format::Pades => Some(SignatureFormat::Pades),
        Format::Auto if bytes.starts_with(b"%PDF-") => Some(SignatureFormat::Pades),
        _ => None,
    }
}

fn the_certificate_named(
    alias: &str,
    arguments: &[String],
    stores: &dyn CertificateStores,
) -> Result<TokenCertificate, Outcome> {
    let scope = scope_named_by(arguments)
        .map_err(|refusal| Outcome::refused(&Refusal::InvalidStore(refusal)))?;
    let named: Vec<TokenCertificate> = within_the_scope(&scope, stores)
        .map_err(|failure| failure_of_the_scope(&failure))?
        .into_iter()
        .filter(|certificate| certificate.reference().label() == alias)
        .collect();
    let Some(first) = named.first() else {
        return Err(Outcome::failed(format!(
            "rfirma: ningún almacén tiene un certificado con el alias «{alias}»; \
             «rfirma listaliases» los enumera"
        )));
    };
    if !named
        .iter()
        .all(|certificate| certificate.is_a_copy_of(first))
    {
        return Err(Outcome::failed(format!(
            "rfirma: hay varios certificados con el alias «{alias}»; acota el almacén con -store"
        )));
    }
    let installed = named
        .iter()
        .find(|certificate| certificate.reference().store().class() == StoreClass::Installed);
    Ok(installed.unwrap_or(first).clone())
}

fn failure_of_the_scope(failure: &ScopeFailure) -> Outcome {
    match failure {
        ScopeFailure::Token(error) => Outcome::failed(format!(
            "rfirma: no se ha podido abrir ningún almacén de certificados ({})",
            error.detail()
        )),
        ScopeFailure::ModuleNotDiscovered(library) => Outcome::failed(format!(
            "rfirma: «{library}» no es un módulo PKCS#11 de los que rfirma ha descubierto, \
             y no carga ninguno solo porque lo nombre la orden"
        )),
    }
}

#[cfg(test)]
mod tests;
