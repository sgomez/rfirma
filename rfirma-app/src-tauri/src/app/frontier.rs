//! Frontera de traducción de errores y situaciones internas a códigos de protocolo hacia la sede.

use crate::app::cycle::CycleError;
use crate::app::signing::CycleFailure;
use crate::channel::Situation as ChannelSituation;
use crate::destination::Situation as DestinationSituation;
use crate::ffi::BridgeError;
use crate::memory::Situation as MemorySituation;
use crate::pkcs11::Situation as TokenSituation;
use crate::protocol::{SafCode, WireAnswer};
use crate::rubric::Situation as RubricSituation;
use crate::signing::Refusal as Inadmissible;

/// Mapea situaciones del token a códigos de error del protocolo.
pub fn code_of_token(situation: TokenSituation) -> SafCode {
    match situation {
        TokenSituation::IncorrectPin
        | TokenSituation::ExpiredSession
        | TokenSituation::Pkcs12Unreadable
        | TokenSituation::Unknown => SafCode::CannotAccessKeystore,
        TokenSituation::PinLocked => SafCode::LockedKeystore,
        TokenSituation::TokenAbsent | TokenSituation::ModuleNotFound => SafCode::CannotFindKeystore,
        TokenSituation::CertificateNotFound => SafCode::NoCertificatesInKeystore,
        TokenSituation::KeyNotRsa => SafCode::IncompatibleKeyType,
    }
}

/// Mapea situaciones de persistencia a códigos de error del protocolo.
pub fn code_of_memory(situation: MemorySituation) -> SafCode {
    match situation {
        MemorySituation::Unreadable => SafCode::CannotReadData,
        MemorySituation::Unwritable => SafCode::CannotSaveData,
    }
}

/// Mapea situaciones de la carpeta destino a códigos de error del protocolo (ADR-0011).
pub fn code_of_destination(situation: DestinationSituation) -> SafCode {
    match situation {
        DestinationSituation::FolderMissing
        | DestinationSituation::NotAFolder
        | DestinationSituation::FolderUnreadable
        | DestinationSituation::NoFreeName => SafCode::CannotSaveData,
    }
}

/// Mapea situaciones del almacén de rúbrica a códigos de error del protocolo (ADR-0012).
pub fn code_of_rubric(situation: RubricSituation) -> SafCode {
    match situation {
        RubricSituation::NotAnAcceptedImage
        | RubricSituation::DamagedImage
        | RubricSituation::ImageTooLarge
        | RubricSituation::SourceUnreadable
        | RubricSituation::StoreUnwritable
        | RubricSituation::StoreUnreadable => SafCode::VisibleSignature,
    }
}

/// Mapea situaciones del canal local a códigos de error del protocolo.
pub fn code_of_channel(situation: ChannelSituation) -> SafCode {
    match situation {
        ChannelSituation::NoDrawnPortIsFree | ChannelSituation::NotListening => {
            SafCode::CannotOpenSocket
        }
        ChannelSituation::MaterialNotUsable => SafCode::CannotAccessSslKeystore,
    }
}

/// Mapea motivos de inadmisibilidad del documento a códigos de error del protocolo.
pub fn code_of_inadmissible(refusal: Inadmissible) -> SafCode {
    match refusal {
        Inadmissible::NotAPdf => SafCode::InvalidPdf,
        Inadmissible::Encrypted => SafCode::PdfWrongPassword,
        Inadmissible::Certified => SafCode::PdfCertified,
    }
}

/// Mapea errores del puente nativo Java a códigos de error del protocolo.
pub fn code_of_bridge(error: &BridgeError) -> SafCode {
    match error {
        BridgeError::PdfHasUnregisteredSignatures(_) => SafCode::ConfirmationNeeded,
        BridgeError::IncompatiblePolicy(_) => SafCode::InvalidPolicy,
        BridgeError::ExecutablePathUnknown(_)
        | BridgeError::NotFound(_)
        | BridgeError::Load { .. }
        | BridgeError::MissingSymbol { .. }
        | BridgeError::IsolateFailed(_)
        | BridgeError::InvalidArgument(_)
        | BridgeError::NullResponse
        | BridgeError::MalformedResponse(_)
        | BridgeError::Failed(_) => SafCode::SignatureFailed,
    }
}

/// Código devuelto cuando el sello de sesión entre prefirma y postfirma no coincide (ADR-0016).
pub fn code_of_broken_seal() -> SafCode {
    SafCode::PostprocessingData
}

/// Mapea un fallo del ciclo trifásico de firma al código de protocolo correspondiente.
pub fn code_of_cycle(failure: &CycleFailure) -> SafCode {
    match failure {
        CycleFailure::DocumentUnreadable(_) => SafCode::CannotReadData,
        CycleFailure::Cycle(CycleError::Inadmissible(refusal)) => code_of_inadmissible(*refusal),
        CycleFailure::Cycle(CycleError::Bridge(error)) => code_of_bridge(error),
        CycleFailure::Cycle(CycleError::Token(error)) => code_of_token(error.situation()),
        CycleFailure::Cycle(CycleError::Seal(_)) => code_of_broken_seal(),
        CycleFailure::SecretOnTheReaderKeypad(_) => SafCode::CannotAccessKeystore,
        CycleFailure::Gone(_) => SafCode::SignatureFailed,
    }
}

/// Construye la respuesta de cancelación voluntaria por parte del usuario.
pub fn cancelled() -> WireAnswer {
    WireAnswer::Cancelled
}

#[cfg(test)]
mod tests;
