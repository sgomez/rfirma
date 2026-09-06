//! La única traducción de las situaciones de la firma local: a la vista de la ventana y al código de la sede (ADR-0009).

use crate::commands::Failure;
use crate::documents::adapters::failures::code_of_document;
use crate::identity::adapters::failures::{code_of_secret_on_the_reader_keypad, code_of_token};
use crate::signing::application::cycle::CycleError;
use crate::signing::application::filtering::FilteringError;
use crate::signing::application::session::CycleFailure;
use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::memory_error::{MemoryError, Situation as MemorySituation};
use crate::signing::domain::{PlacementError, Refusal, SealMismatch};
use crate::site::domain::protocol::SafCode;

fn memory_told(situation: MemorySituation) -> (&'static str, SafCode) {
    match situation {
        MemorySituation::Unreadable => ("settingsUnreadable", SafCode::CannotReadData),
        MemorySituation::Unwritable => ("settingsUnwritable", SafCode::CannotSaveData),
    }
}

/// Código de protocolo de una situación de persistencia (ADR-0010).
pub fn code_of_memory(situation: MemorySituation) -> SafCode {
    memory_told(situation).1
}

impl From<MemoryError> for Failure {
    fn from(error: MemoryError) -> Self {
        Self::new(memory_told(error.situation()).0, error.detail().to_owned())
    }
}

/// Código de protocolo de un documento que no se puede firmar.
pub fn code_of_inadmissible(refusal: Refusal) -> SafCode {
    match refusal {
        Refusal::NotAPdf => SafCode::InvalidPdf,
        Refusal::Encrypted => SafCode::PdfWrongPassword,
        Refusal::Certified => SafCode::PdfCertified,
    }
}

impl From<Refusal> for Failure {
    fn from(refusal: Refusal) -> Self {
        Self::new(refusal.situation(), refusal.to_string())
    }
}

/// Código devuelto cuando el sello de sesión entre prefirma y postfirma no coincide (ADR-0016).
pub fn code_of_broken_seal() -> SafCode {
    SafCode::PostprocessingData
}

impl From<SealMismatch> for Failure {
    fn from(error: SealMismatch) -> Self {
        Self::new("sealMismatch", error.to_string())
    }
}

fn bridge_told(error: &BridgeError) -> (&'static str, SafCode) {
    match error {
        BridgeError::PdfHasUnregisteredSignatures(_) => {
            ("pdfHasUnregisteredSignatures", SafCode::ConfirmationNeeded)
        }
        BridgeError::IncompatiblePolicy(_) => ("bridgeFailed", SafCode::InvalidPolicy),
        BridgeError::ExecutablePathUnknown(_)
        | BridgeError::NotFound(_)
        | BridgeError::Load { .. }
        | BridgeError::MissingSymbol { .. }
        | BridgeError::IsolateFailed(_)
        | BridgeError::InvalidArgument(_)
        | BridgeError::NullResponse
        | BridgeError::MalformedResponse(_)
        | BridgeError::Failed(_) => ("bridgeFailed", SafCode::SignatureFailed),
    }
}

/// Código de protocolo de un error del puente nativo.
pub fn code_of_bridge(error: &BridgeError) -> SafCode {
    bridge_told(error).1
}

impl From<&BridgeError> for Failure {
    fn from(error: &BridgeError) -> Self {
        Self::new(bridge_told(error).0, error.to_string())
    }
}

impl From<BridgeError> for Failure {
    fn from(error: BridgeError) -> Self {
        Self::from(&error)
    }
}

impl From<IsolateGone> for Failure {
    fn from(error: IsolateGone) -> Self {
        Self::new("bridgeFailed", error.to_string())
    }
}

fn placement_told(error: &PlacementError) -> (&'static str, SafCode) {
    match error {
        PlacementError::OutOfDocument(_) => ("pageOutOfDocument", SafCode::VisibleSignature),
        PlacementError::BadRotation(_) => ("unknown", SafCode::VisibleSignature),
        PlacementError::OutOfPage(_) => ("boxOutOfPage", SafCode::VisibleSignature),
    }
}

impl From<PlacementError> for Failure {
    fn from(error: PlacementError) -> Self {
        Self::new(placement_told(&error).0, error.to_string())
    }
}

fn cycle_error_told(error: &CycleError) -> (Failure, SafCode) {
    match error {
        CycleError::Inadmissible(refusal) => {
            (Failure::from(*refusal), code_of_inadmissible(*refusal))
        }
        CycleError::Bridge(error) => (Failure::from(error), code_of_bridge(error)),
        CycleError::Token(error) => (
            Failure::from(error.clone()),
            code_of_token(error.situation()),
        ),
        CycleError::Seal(error) => (Failure::from(*error), code_of_broken_seal()),
    }
}

impl From<&CycleError> for Failure {
    fn from(error: &CycleError) -> Self {
        cycle_error_told(error).0
    }
}

impl From<CycleError> for Failure {
    fn from(error: CycleError) -> Self {
        Self::from(&error)
    }
}

fn cycle_told(failure: &CycleFailure) -> (Failure, SafCode) {
    match failure {
        CycleFailure::Document(error) => (Failure::from(error.clone()), code_of_document(error)),
        CycleFailure::Placement(error) => (Failure::from(error.clone()), placement_told(error).1),
        CycleFailure::Cycle(error) => cycle_error_told(error),
        CycleFailure::SecretOnTheReaderKeypad(refusal) => (
            Failure::from(*refusal),
            code_of_secret_on_the_reader_keypad(),
        ),
        CycleFailure::Gone(gone) => (Failure::from(*gone), SafCode::SignatureFailed),
        CycleFailure::NoOpenCycle => (
            Failure::new("unknown", "no hay ninguna firma empezada"),
            SafCode::SignatureFailed,
        ),
        CycleFailure::NotSignedYet => (
            Failure::new("unknown", "todavía no se ha firmado en el token"),
            SafCode::SignatureFailed,
        ),
        CycleFailure::NoSignedDocument => (
            Failure::new("unknown", "no hay ningun documento firmado en esta sesion"),
            SafCode::SignatureFailed,
        ),
    }
}

/// Código de protocolo de un fallo del ciclo trifásico.
pub fn code_of_cycle(failure: &CycleFailure) -> SafCode {
    cycle_told(failure).1
}

impl From<&CycleFailure> for Failure {
    fn from(failure: &CycleFailure) -> Self {
        cycle_told(failure).0
    }
}

impl From<CycleFailure> for Failure {
    fn from(failure: CycleFailure) -> Self {
        Self::from(&failure)
    }
}

impl From<&FilteringError> for Failure {
    fn from(error: &FilteringError) -> Self {
        match error {
            FilteringError::Token(error) => error.clone().into(),
            FilteringError::Engine(error) => error.into(),
            FilteringError::EngineOutOfRange(index) => Self::new(
                "bridgeFailed",
                format!("el motor de filtros ha devuelto el indice {index}"),
            ),
            FilteringError::ExcludedByTheSite(label) => Self::new(
                "certificateNotFound",
                format!("la sede excluye {label}: su filtro ya no lo acepta"),
            ),
        }
    }
}

impl From<FilteringError> for Failure {
    fn from(error: FilteringError) -> Self {
        Self::from(&error)
    }
}

#[cfg(test)]
mod tests;
