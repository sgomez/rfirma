//! El caso de uso de la línea de órdenes (ADR-0041): atiende una orden y dice qué sale por cada flujo, sin escribir en ninguno.

use std::path::Path;

use crate::desktop::application::store_scope::{within_the_scope, ScopeFailure};
use crate::desktop::domain::command_line::{
    command_of, handover_to_the_window, is_a_help_flag, normalised, parameter_left_out,
    verbose_outside_verify, Command, Refusal, WindowHandover, JSON, PASSWORD_FD, XML,
};
use crate::desktop::domain::platform::Platform;
use crate::desktop::domain::sign_arguments::{
    parse_sign_arguments, Format, Selection, SignArguments,
};
use crate::desktop::domain::store_scope::scope_named_by;
use crate::desktop::ports::{
    CertificateFilter, CertificateStores, CommandLineFiles, CommandLineSigning, DesktopHandover,
    DocumentSigner, GraphicalPicker, LocalTimeZone, SecretDescriptor, SignatureReading,
    SignatureVerifier, Terminal,
};
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::store::StoreClass;
use crate::signing::domain::bridge::{Format as SignatureFormat, SignatureOperation, XadesVariant};
use crate::signing::domain::Language;
use crate::site::domain::protocol::detection::{shape_of, DetectedShape};
use crate::site::domain::protocol::{site_filter, SiteFilter};

mod certgui;
mod certtui;
mod config;
mod json_output;
mod response;
mod signing;
mod verify;
use json_output::{ListedAliases, SignedDocument};
use response::{Field, Response};
use signing::{in_the_xml_response, sign};
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
    /// Los descriptores de los que sale el PIN de `-password-fd`.
    pub descriptor: &'a dyn SecretDescriptor,
    /// El proceso de escritorio que recibe los ficheros de `-gui`.
    pub desktop: &'a dyn DesktopHandover,
    /// El filtro de certificados de la sede.
    pub filter: &'a dyn CertificateFilter,
    /// Los ficheros que se leen y se escriben.
    pub files: &'a dyn CommandLineFiles,
    /// El validador del original.
    pub verifier: &'a dyn SignatureVerifier,
    /// La lectura de las firmas que enseña `verify -v`.
    pub reader: &'a dyn SignatureReading,
    /// La zona horaria en la que se enseña la fecha declarada.
    pub time_zone: &'a dyn LocalTimeZone,
    /// El idioma del sistema, en el que se escriben los textos del catálogo.
    pub language: Language,
    /// El sistema en el que corre, del que dependen `-store` y los textos.
    pub platform: Platform,
    /// Quien firma por el camino de la sede.
    pub signer: &'a dyn DocumentSigner,
    /// La ventana de sede en la que se elige con `-certgui`.
    pub window: &'a dyn GraphicalPicker,
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
    let arguments = &normalised(arguments);
    let command = match command_of(arguments, ports.platform) {
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
            stdout: command
                .syntax(ports.platform)
                .unwrap_or_default()
                .as_bytes()
                .to_vec(),
            stderr: Vec::new(),
        };
    }
    if let Some(refusal) = parameter_left_out(arguments) {
        return Outcome::refused(&refusal);
    }
    if let Some(refusal) = verbose_outside_verify(command, arguments) {
        return Outcome::refused(&refusal);
    }
    if arguments.iter().any(|argument| argument == XML)
        && arguments.iter().any(|argument| argument == JSON)
    {
        return Outcome::refused(&Refusal::JsonWithXml);
    }
    let signing = if matches!(command, Command::Sign | Command::Cosign) {
        match parse_sign_arguments(&arguments[1..]) {
            Ok(parsed) => Some(parsed),
            Err(refusal) => return Outcome::refused(&Refusal::InvalidArguments(refusal)),
        }
    } else {
        None
    };
    match handover_to_the_window(command, arguments) {
        Err(refusal) => Outcome::refused(&refusal),
        Ok(Some(handover)) => hand_over_to_the_window(ports.desktop, handover),
        Ok(None) => carried_out(command, arguments, signing.as_ref(), ports),
    }
}

fn hand_over_to_the_window(desktop: &dyn DesktopHandover, handover: WindowHandover) -> Outcome {
    match desktop.hand_over(Path::new(handover.file), handover.intent) {
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
        (Command::ListAliases, _) => list_aliases(arguments, ports),
        (Command::Verify, _) => verify::verify(arguments, ports),
        (Command::Sign, Some(parsed)) => sign(arguments, parsed, SignatureOperation::Sign, ports),
        (Command::Cosign, Some(parsed)) => {
            sign(arguments, parsed, SignatureOperation::Cosign, ports)
        }
        _ => Outcome::not_yet_available(&format!("la orden «{}»", command.name())),
    }
}

fn list_aliases(arguments: &[String], ports: &CommandLinePorts) -> Outcome {
    match (
        aliases_listed(arguments, ports),
        document_asked_by(arguments),
    ) {
        (Ok(certificates), Some(Asked::Json)) => Outcome {
            stdout: json_output::compact(&ListedAliases::of(&certificates)),
            ..Outcome::aliases_of(&certificates)
        },
        (Ok(certificates), Some(Asked::Xml)) => Outcome {
            stdout: aliases_response(&certificates).to_xml(),
            ..Outcome::aliases_of(&certificates)
        },
        (Ok(certificates), None) => Outcome::aliases_of(&certificates),
        (Err(failed), Some(Asked::Xml)) => in_the_xml_response(failed, None),
        (Err(failed), _) => failed,
    }
}

/// El documento que pide la orden en stdout, en lugar del texto.
#[derive(Clone, Copy)]
enum Asked {
    Xml,
    Json,
}

fn document_asked_by(arguments: &[String]) -> Option<Asked> {
    let asks = |parameter: &str| arguments.iter().any(|argument| argument == parameter);
    if asks(XML) {
        Some(Asked::Xml)
    } else if asks(JSON) {
        Some(Asked::Json)
    } else {
        None
    }
}

fn aliases_listed(
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<Vec<TokenCertificate>, Outcome> {
    if arguments.iter().any(|argument| argument == PASSWORD_FD) {
        return Err(Outcome::refused(&Refusal::PasswordForListing));
    }
    let scope = scope_named_by(arguments, ports.platform)
        .map_err(|refusal| Outcome::refused(&Refusal::InvalidStore(refusal)))?;
    within_the_scope(&scope, ports.stores).map_err(|failure| failure_of_the_scope(&failure))
}

fn aliases_response(certificates: &[TokenCertificate]) -> Response {
    let aliases = certificates
        .iter()
        .map(|certificate| certificate.reference().label().to_owned())
        .collect();
    Response::new("ok", vec![Field::Many("alias", aliases)])
}

fn the_certificate_chosen_by(
    selection: &Selection,
    input: &Path,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<(TokenCertificate, Option<ProtectedSecret>), Outcome> {
    let without_a_secret = |certificate| (certificate, None);
    match selection {
        Selection::Alias(alias) => {
            the_certificate_named(alias, arguments, ports).map(without_a_secret)
        }
        Selection::Filter(expression) => {
            the_only_certificate_accepted_by(expression, arguments, ports).map(without_a_secret)
        }
        Selection::Terminal { filter } => {
            certtui::the_certificate_chosen_on_the_terminal(filter.as_deref(), arguments, ports)
                .map(without_a_secret)
        }
        Selection::Window { filter } => certgui::the_certificate_chosen_in_the_window(
            filter.as_deref(),
            input,
            arguments,
            ports,
        ),
    }
}

fn the_certificate_named(
    alias: &str,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<TokenCertificate, Outcome> {
    let named: Vec<TokenCertificate> = listed_within_the_store(arguments, ports)?
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
            "rfirma: hay varios certificados con el alias «{alias}»; acota el almacén con --store"
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
    let filter = the_site_filter_of(expression)?;
    let listed = listed_within_the_store(arguments, ports)?;
    let accepted = accepted_by(&filter, listed, ports)?;
    let Some(first) = accepted.first() else {
        return Err(Outcome::failed(format!(
            "rfirma: ningún certificado cumple el filtro «{expression}»"
        )));
    };
    one_copy_of(&accepted, first).ok_or_else(|| {
        Outcome::failed(format!(
            "rfirma: varios certificados cumplen el filtro «{expression}»; \
             afínalo o acota el almacén con --store"
        ))
    })
}

fn the_site_filter_of(expression: &str) -> Result<SiteFilter, Outcome> {
    let filter = site_filter(&[("filters".to_owned(), expression.to_owned())]);
    if filter.declares_nothing() {
        return Err(Outcome::failed(format!(
            "rfirma: el filtro «{expression}» no nombra ningún criterio de los que reconoce la sede"
        )));
    }
    Ok(filter)
}

fn listed_within_the_store(
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<Vec<TokenCertificate>, Outcome> {
    let scope = scope_named_by(arguments, ports.platform)
        .map_err(|refusal| Outcome::refused(&Refusal::InvalidStore(refusal)))?;
    within_the_scope(&scope, ports.stores).map_err(|failure| failure_of_the_scope(&failure))
}

fn accepted_by(
    filter: &SiteFilter,
    listed: Vec<TokenCertificate>,
    ports: &CommandLinePorts,
) -> Result<Vec<TokenCertificate>, Outcome> {
    ports
        .filter
        .accepted(filter, listed)
        .map_err(|reason| Outcome::failed(format!("rfirma: no se ha podido filtrar ({reason})")))
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
