use super::*;
use crate::identity::domain::error::{Situation as TokenSituation, TokenError};

#[test]
fn the_three_pdf_situations_get_three_different_codes() {
    assert_eq!(code_of_inadmissible(Refusal::NotAPdf), SafCode::InvalidPdf);
    assert_eq!(
        code_of_inadmissible(Refusal::Encrypted),
        SafCode::PdfWrongPassword
    );
    assert_eq!(
        code_of_inadmissible(Refusal::Certified),
        SafCode::PdfCertified
    );
}

#[test]
fn a_pdf_with_unregistered_signatures_asks_for_confirmation() {
    let error = BridgeError::PdfHasUnregisteredSignatures("da igual el texto".to_owned());

    assert_eq!(code_of_bridge(&error), SafCode::ConfirmationNeeded);
    assert_eq!(
        Failure::from(error).situation,
        "pdfHasUnregisteredSignatures"
    );
    assert_eq!(
        code_of_bridge(&BridgeError::Failed("otra cosa".to_owned())),
        SafCode::SignatureFailed
    );
}

#[test]
fn what_breaks_after_the_consent_keeps_its_own_code_and_its_own_name() {
    for (failure, code, name) in [
        (
            CycleFailure::Cycle(CycleError::Token(TokenError::new(
                TokenSituation::TokenAbsent,
                "no hay tarjeta",
            ))),
            SafCode::CannotFindKeystore,
            "tokenAbsent",
        ),
        (
            CycleFailure::Cycle(CycleError::Bridge(BridgeError::IncompatiblePolicy(
                "la politica de la sede".to_owned(),
            ))),
            SafCode::InvalidPolicy,
            "bridgeFailed",
        ),
        (
            CycleFailure::Cycle(CycleError::Inadmissible(Refusal::Encrypted)),
            SafCode::PdfWrongPassword,
            "documentEncrypted",
        ),
        (
            CycleFailure::Document(crate::documents::domain::error::DocumentError::Unreadable(
                "ya no esta".to_owned(),
            )),
            SafCode::CannotReadData,
            "documentUnreadable",
        ),
        (
            CycleFailure::Cycle(CycleError::Bridge(BridgeError::Failed(
                "lo que dijera Java".to_owned(),
            ))),
            SafCode::SignatureFailed,
            "bridgeFailed",
        ),
        (
            CycleFailure::NoOpenCycle,
            SafCode::SignatureFailed,
            "unknown",
        ),
    ] {
        let told = cycle_told(&failure);
        assert_eq!(told.1, code, "{failure:?}");
        assert_eq!(told.0.situation, name, "{failure:?}");
    }
}

#[test]
fn a_broken_session_seal_is_not_a_signature_that_did_not_come_out() {
    let broken = CycleFailure::Cycle(CycleError::Seal(SealMismatch));

    assert_eq!(code_of_cycle(&broken), SafCode::PostprocessingData);
    assert_eq!(code_of_cycle(&broken), code_of_broken_seal());
    assert_eq!(Failure::from(broken).situation, "sealMismatch");
}

#[test]
fn the_memory_situations_have_their_camel_case_names_and_their_codes() {
    assert_eq!(
        memory_told(MemorySituation::Unreadable),
        ("settingsUnreadable", SafCode::CannotReadData)
    );
    assert_eq!(
        memory_told(MemorySituation::Unwritable),
        ("settingsUnwritable", SafCode::CannotSaveData)
    );
}
