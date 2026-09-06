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
mod tests {
    use super::*;

    fn every_code_of_ours() -> Vec<SafCode> {
        let mut codes = Vec::new();
        for situation in [
            TokenSituation::IncorrectPin,
            TokenSituation::PinLocked,
            TokenSituation::TokenAbsent,
            TokenSituation::ExpiredSession,
            TokenSituation::ModuleNotFound,
            TokenSituation::CertificateNotFound,
            TokenSituation::Pkcs12Unreadable,
            TokenSituation::KeyNotRsa,
            TokenSituation::Unknown,
        ] {
            codes.push(code_of_token(situation));
        }
        for situation in [MemorySituation::Unreadable, MemorySituation::Unwritable] {
            codes.push(code_of_memory(situation));
        }
        for situation in [
            DestinationSituation::FolderMissing,
            DestinationSituation::NotAFolder,
            DestinationSituation::FolderUnreadable,
            DestinationSituation::NoFreeName,
        ] {
            codes.push(code_of_destination(situation));
        }
        for situation in [
            RubricSituation::NotAnAcceptedImage,
            RubricSituation::DamagedImage,
            RubricSituation::ImageTooLarge,
            RubricSituation::SourceUnreadable,
            RubricSituation::StoreUnwritable,
            RubricSituation::StoreUnreadable,
        ] {
            codes.push(code_of_rubric(situation));
        }
        for situation in [
            ChannelSituation::NoDrawnPortIsFree,
            ChannelSituation::MaterialNotUsable,
            ChannelSituation::NotListening,
        ] {
            codes.push(code_of_channel(situation));
        }
        for refusal in [
            Inadmissible::NotAPdf,
            Inadmissible::Encrypted,
            Inadmissible::Certified,
        ] {
            codes.push(code_of_inadmissible(refusal));
        }
        codes.push(code_of_bridge(&BridgeError::Failed(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_bridge(&BridgeError::PdfHasUnregisteredSignatures(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_bridge(&BridgeError::IncompatiblePolicy(
            "lo que dijera Java".to_owned(),
        )));
        codes.push(code_of_broken_seal());
        codes
    }

    #[test]
    fn every_situation_of_ours_lands_inside_the_published_catalogue() {
        for code in every_code_of_ours() {
            assert!(
                SafCode::ALL.contains(&code),
                "{code:?} no esta en el catalogo publicado"
            );
            let line = WireAnswer::refused(code).on_the_wire();
            assert!(
                line.starts_with("SAF_") && line.len() > 4,
                "«{line}» no la lee el cliente publicado como un error"
            );
        }
    }

    #[test]
    fn the_shadow_attack_code_is_never_produced() {
        assert!(
            !every_code_of_ours().contains(&SafCode::PdfShadowAttack),
            "SAF_48 no existe en la 1.9.2 y no puede salir de aqui"
        );
    }

    #[test]
    fn no_refusal_of_ours_is_a_cancellation() {
        for code in every_code_of_ours() {
            let answer = WireAnswer::refused(code);
            assert_ne!(answer, WireAnswer::Cancelled);
            assert_ne!(answer.on_the_wire(), "CANCEL");
        }
        assert_eq!(cancelled().on_the_wire(), "CANCEL");
    }

    #[test]
    fn the_three_pdf_situations_get_three_different_codes() {
        assert_eq!(
            code_of_inadmissible(Inadmissible::NotAPdf),
            SafCode::InvalidPdf
        );
        assert_eq!(
            code_of_inadmissible(Inadmissible::Encrypted),
            SafCode::PdfWrongPassword
        );
        assert_eq!(
            code_of_inadmissible(Inadmissible::Certified),
            SafCode::PdfCertified
        );
    }

    #[test]
    fn a_pdf_with_unregistered_signatures_asks_for_confirmation() {
        let error = BridgeError::PdfHasUnregisteredSignatures("da igual el texto".to_owned());

        assert_eq!(code_of_bridge(&error), SafCode::ConfirmationNeeded);
        assert_eq!(
            code_of_bridge(&BridgeError::Failed("otra cosa".to_owned())),
            SafCode::SignatureFailed
        );
    }

    #[test]
    fn what_breaks_after_the_consent_keeps_its_own_code() {
        for (failure, expected) in [
            (
                CycleFailure::Cycle(CycleError::Token(crate::pkcs11::TokenError::new(
                    TokenSituation::TokenAbsent,
                    "no hay tarjeta",
                ))),
                SafCode::CannotFindKeystore,
            ),
            (
                CycleFailure::Cycle(CycleError::Bridge(BridgeError::IncompatiblePolicy(
                    "la politica de la sede".to_owned(),
                ))),
                SafCode::InvalidPolicy,
            ),
            (
                CycleFailure::Cycle(CycleError::Inadmissible(Inadmissible::Encrypted)),
                SafCode::PdfWrongPassword,
            ),
            (
                CycleFailure::DocumentUnreadable("ya no esta".to_owned()),
                SafCode::CannotReadData,
            ),
            (
                CycleFailure::Cycle(CycleError::Bridge(BridgeError::Failed(
                    "lo que dijera Java".to_owned(),
                ))),
                SafCode::SignatureFailed,
            ),
        ] {
            assert_eq!(code_of_cycle(&failure), expected, "{failure:?}");
        }
    }

    #[test]
    fn a_broken_session_seal_is_not_a_signature_that_did_not_come_out() {
        let broken = CycleFailure::Cycle(CycleError::Seal(crate::signing::SealMismatch));

        assert_eq!(code_of_cycle(&broken), SafCode::PostprocessingData);
        assert_eq!(code_of_cycle(&broken), code_of_broken_seal());
    }
}
