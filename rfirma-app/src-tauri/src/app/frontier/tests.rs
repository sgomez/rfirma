use super::*;

fn every_token_code() -> Vec<SafCode> {
    [
        TokenSituation::IncorrectPin,
        TokenSituation::PinLocked,
        TokenSituation::TokenAbsent,
        TokenSituation::ExpiredSession,
        TokenSituation::ModuleNotFound,
        TokenSituation::CertificateNotFound,
        TokenSituation::Pkcs12Unreadable,
        TokenSituation::KeyNotRsa,
        TokenSituation::Unknown,
    ]
    .into_iter()
    .map(code_of_token)
    .collect()
}

fn every_memory_code() -> Vec<SafCode> {
    [MemorySituation::Unreadable, MemorySituation::Unwritable]
        .into_iter()
        .map(code_of_memory)
        .collect()
}

fn every_destination_code() -> Vec<SafCode> {
    [
        DestinationSituation::FolderMissing,
        DestinationSituation::NotAFolder,
        DestinationSituation::FolderUnreadable,
        DestinationSituation::NoFreeName,
    ]
    .into_iter()
    .map(code_of_destination)
    .collect()
}

fn every_rubric_code() -> Vec<SafCode> {
    [
        RubricSituation::NotAnAcceptedImage,
        RubricSituation::DamagedImage,
        RubricSituation::ImageTooLarge,
        RubricSituation::SourceUnreadable,
        RubricSituation::StoreUnwritable,
        RubricSituation::StoreUnreadable,
    ]
    .into_iter()
    .map(code_of_rubric)
    .collect()
}

fn every_channel_code() -> Vec<SafCode> {
    [
        ChannelSituation::NoDrawnPortIsFree,
        ChannelSituation::MaterialNotUsable,
        ChannelSituation::NotListening,
    ]
    .into_iter()
    .map(code_of_channel)
    .collect()
}

fn every_inadmissible_code() -> Vec<SafCode> {
    [
        Inadmissible::NotAPdf,
        Inadmissible::Encrypted,
        Inadmissible::Certified,
    ]
    .into_iter()
    .map(code_of_inadmissible)
    .collect()
}

fn every_bridge_code() -> Vec<SafCode> {
    [
        BridgeError::Failed("lo que dijera Java".to_owned()),
        BridgeError::PdfHasUnregisteredSignatures("lo que dijera Java".to_owned()),
        BridgeError::IncompatiblePolicy("lo que dijera Java".to_owned()),
    ]
    .iter()
    .map(code_of_bridge)
    .collect()
}

fn every_code_of_ours() -> Vec<SafCode> {
    [
        every_token_code(),
        every_memory_code(),
        every_destination_code(),
        every_rubric_code(),
        every_channel_code(),
        every_inadmissible_code(),
        every_bridge_code(),
        vec![code_of_broken_seal()],
    ]
    .concat()
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
