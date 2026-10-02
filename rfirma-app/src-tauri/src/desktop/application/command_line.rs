//! El caso de uso de la línea de órdenes: atiende una orden y dice qué sale por cada flujo, sin escribir en ninguno.

use std::path::Path;

use base64::Engine as _;

use crate::desktop::application::store_scope::{within_the_scope, ScopeFailure};
use crate::desktop::domain::command_line::{
    command_of, file_for_the_window, is_a_help_flag, parameter_left_out, Command, Refusal,
};
use crate::desktop::domain::sign_arguments::{
    parse_sign_arguments, Format, Selection, SignArguments,
};
use crate::desktop::domain::store_scope::scope_named_by;
use crate::desktop::ports::{
    CertificateFilter, CertificateStores, CommandLineFiles, CommandLineSigning, DesktopHandover,
    DocumentSigner, SignatureVerifier, Terminal,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::StoreClass;
use crate::signing::domain::bridge::Format as SignatureFormat;
use crate::site::domain::protocol::site_filter;

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
    /// El filtro de certificados de la sede.
    pub filter: &'a dyn CertificateFilter,
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
    let Signed(outcome, document) = signed(arguments, parsed, ports);
    if parsed.xml {
        return in_the_xml_response(outcome, document.as_deref());
    }
    outcome
}

/// El desenlace de firmar y, si no se escribió en `-o`, el documento firmado para la respuesta XML.
struct Signed(Outcome, Option<Vec<u8>>);

fn signed(arguments: &[String], parsed: &SignArguments, ports: &CommandLinePorts) -> Signed {
    let outcome = |outcome: Outcome| Signed(outcome, None);
    let Some(selection) = &parsed.selection else {
        return outcome(Outcome::not_yet_available(
            "elegir el certificado sin -alias",
        ));
    };
    if let Some(parameter) = not_yet_available_in(parsed) {
        return outcome(Outcome::not_yet_available(parameter));
    }
    let certificate = match the_certificate_chosen_by(selection, arguments, ports) {
        Ok(certificate) => certificate,
        Err(failed) => return outcome(failed),
    };
    let input = Path::new(&parsed.input);
    let bytes = match ports.files.read(input) {
        Ok(bytes) => bytes,
        Err(reason) => {
            return outcome(Outcome::failed(format!(
                "rfirma: no se puede leer «{}» ({reason})",
                parsed.input
            )))
        }
    };
    let Some(format) = signature_format_of(parsed.format, &bytes) else {
        return outcome(Outcome::not_yet_available("la firma CAdES y XAdES"));
    };
    let document = match ports.signer.sign(&CommandLineSigning {
        input,
        certificate: &certificate,
        format,
        algorithm: parsed.algorithm,
    }) {
        Ok(document) => document,
        Err(reason) => {
            return outcome(Outcome::failed(format!(
                "rfirma: no se ha podido firmar ({reason})"
            )))
        }
    };
    let Some(output) = &parsed.output else {
        ports.signer.remember(&certificate);
        return Signed(
            Outcome {
                exit_code: SUCCEEDED,
                stdout: Vec::new(),
                stderr: vec!["rfirma: firma generada".to_owned()],
            },
            Some(document),
        );
    };
    if let Err(reason) = ports.files.write(Path::new(output), &document) {
        return outcome(Outcome::failed(format!(
            "rfirma: no se puede escribir «{output}» ({reason})"
        )));
    }
    ports.signer.remember(&certificate);
    outcome(Outcome {
        exit_code: SUCCEEDED,
        stdout: Vec::new(),
        stderr: vec![format!("rfirma: firma guardada en «{output}»")],
    })
}

fn in_the_xml_response(outcome: Outcome, signature: Option<&[u8]>) -> Outcome {
    let message = outcome
        .stderr
        .iter()
        .map(|line| line.strip_prefix("rfirma: ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join(" ");
    Outcome {
        stdout: xml_response(outcome.exit_code == SUCCEEDED, &message, signature).into_bytes(),
        ..outcome
    }
}

fn xml_response(succeeded: bool, message: &str, signature: Option<&[u8]>) -> String {
    let sign = signature
        .map(|bytes| {
            format!(
                "<sign>{}</sign>",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        })
        .unwrap_or_default();
    format!(
        "<afirma><result>{succeeded}</result><response><msg>{}</msg>{sign}</response></afirma>\n",
        escaped_for_xml(message)
    )
}

fn escaped_for_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn the_certificate_chosen_by(
    selection: &Selection,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<TokenCertificate, Outcome> {
    match selection {
        Selection::Alias(alias) => the_certificate_named(alias, arguments, ports.stores),
        Selection::Filter(expression) => {
            the_only_certificate_accepted_by(expression, arguments, ports)
        }
        Selection::Terminal { .. } => Err(Outcome::not_yet_available("-certtui")),
    }
}

fn not_yet_available_in(parsed: &SignArguments) -> Option<&'static str> {
    if parsed.config.is_some() {
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
    one_copy_of(&named, first).ok_or_else(|| {
        Outcome::failed(format!(
            "rfirma: hay varios certificados con el alias «{alias}»; acota el almacén con -store"
        ))
    })
}

fn one_copy_of(named: &[TokenCertificate], first: &TokenCertificate) -> Option<TokenCertificate> {
    if !named
        .iter()
        .all(|certificate| certificate.is_a_copy_of(first))
    {
        return None;
    }
    let installed = named
        .iter()
        .find(|certificate| certificate.reference().store().class() == StoreClass::Installed);
    Some(installed.unwrap_or(first).clone())
}

fn the_only_certificate_accepted_by(
    expression: &str,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<TokenCertificate, Outcome> {
    let filter = site_filter(&[("filters".to_owned(), expression.to_owned())]);
    if filter.declares_nothing() {
        return Err(Outcome::failed(format!(
            "rfirma: el filtro «{expression}» no nombra ningún criterio de los que reconoce la sede"
        )));
    }
    let scope = scope_named_by(arguments)
        .map_err(|refusal| Outcome::refused(&Refusal::InvalidStore(refusal)))?;
    let listed =
        within_the_scope(&scope, ports.stores).map_err(|failure| failure_of_the_scope(&failure))?;
    let accepted = ports
        .filter
        .accepted(&filter, listed)
        .map_err(|reason| Outcome::failed(format!("rfirma: no se ha podido filtrar ({reason})")))?;
    let Some(first) = accepted.first() else {
        return Err(Outcome::failed(format!(
            "rfirma: ningún certificado cumple el filtro «{expression}»"
        )));
    };
    one_copy_of(&accepted, first).ok_or_else(|| {
        Outcome::failed(format!(
            "rfirma: varios certificados cumplen el filtro «{expression}»; \
             afínalo o acota el almacén con -store"
        ))
    })
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
