use super::*;
use crate::documents::adapters::failures::{code_of_destination, code_of_rubric};
use crate::documents::domain::destination::Situation as DestinationSituation;
use crate::documents::domain::error::DocumentError;
use crate::documents::domain::rubric::Situation as RubricSituation;
use crate::identity::domain::error::{Situation as TokenSituation, TokenError};
use crate::signing::adapters::failures::{code_of_broken_seal, code_of_memory};
use crate::signing::application::cycle::CycleError;
use crate::signing::application::filtering::FilteringError;
use crate::signing::application::session::CycleFailure;
use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::memory_error::Situation as MemorySituation;
use crate::signing::domain::Refusal as Inadmissible;

fn every_refusal_of_the_errand() -> Vec<SiteRefusal> {
    vec![
        SiteRefusal::Token(TokenError::new(
            TokenSituation::TokenAbsent,
            "no hay tarjeta",
        )),
        SiteRefusal::Inadmissible(Inadmissible::Encrypted),
        SiteRefusal::Policies(BridgeError::IncompatiblePolicy("la politica".to_owned())),
        SiteRefusal::CouldNotFilter(FilteringError::EngineOutOfRange(9)),
        SiteRefusal::NoCertificateTheSiteAccepts,
        SiteRefusal::NotUsableForTheSite(FilteringError::ExcludedByTheSite("X".to_owned())),
        SiteRefusal::ScratchFolderMissing("no such directory".to_owned()),
        SiteRefusal::ScratchUnwritable("read-only".to_owned()),
        SiteRefusal::Cycle(CycleFailure::Document(DocumentError::Unreadable(
            "ya no esta".to_owned(),
        ))),
        SiteRefusal::Cycle(CycleFailure::Cycle(CycleError::Seal(
            crate::signing::domain::SealMismatch,
        ))),
        SiteRefusal::Cycle(CycleFailure::NoOpenCycle),
    ]
}

fn every_code_of_ours() -> Vec<SafCode> {
    let mut codes: Vec<SafCode> = every_refusal_of_the_errand().iter().map(code_of).collect();
    codes.extend(
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
        .map(code_of_token),
    );
    codes.extend([MemorySituation::Unreadable, MemorySituation::Unwritable].map(code_of_memory));
    codes.extend(
        [
            DestinationSituation::FolderMissing,
            DestinationSituation::NotAFolder,
            DestinationSituation::FolderUnreadable,
            DestinationSituation::NoFreeName,
        ]
        .map(code_of_destination),
    );
    codes.extend(
        [
            RubricSituation::NotAnAcceptedImage,
            RubricSituation::DamagedImage,
            RubricSituation::ImageTooLarge,
            RubricSituation::SourceUnreadable,
            RubricSituation::StoreUnwritable,
            RubricSituation::StoreUnreadable,
        ]
        .map(code_of_rubric),
    );
    codes.extend(
        [
            ChannelSituation::NoDrawnPortIsFree,
            ChannelSituation::MaterialNotUsable,
            ChannelSituation::NotListening,
        ]
        .map(code_of_channel),
    );
    codes.extend(
        [
            Inadmissible::NotAPdf,
            Inadmissible::Encrypted,
            Inadmissible::Certified,
        ]
        .map(code_of_inadmissible),
    );
    codes.extend(
        [
            BridgeError::Failed("lo que dijera Java".to_owned()),
            BridgeError::PdfHasUnregisteredSignatures("lo que dijera Java".to_owned()),
            BridgeError::IncompatiblePolicy("lo que dijera Java".to_owned()),
        ]
        .iter()
        .map(code_of_bridge),
    );
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
fn the_window_and_the_site_hear_about_each_refusal_from_the_same_match() {
    for (refusal, name, code) in [
        (
            SiteRefusal::NoCertificateTheSiteAccepts,
            "certificateNotFound",
            SafCode::NoCertificatesInKeystore,
        ),
        (
            SiteRefusal::NotUsableForTheSite(FilteringError::EngineOutOfRange(3)),
            "bridgeFailed",
            SafCode::NoCertificatesInKeystore,
        ),
        (
            SiteRefusal::CouldNotFilter(FilteringError::ExcludedByTheSite("X".to_owned())),
            "certificateNotFound",
            SafCode::CannotAccessKeystore,
        ),
        (
            SiteRefusal::ScratchFolderMissing("x".to_owned()),
            "folderMissing",
            SafCode::CannotSaveData,
        ),
        (
            SiteRefusal::Cycle(CycleFailure::Document(DocumentError::no_longer_open())),
            "documentUnreadable",
            SafCode::CannotReadData,
        ),
        (
            SiteRefusal::Cycle(CycleFailure::NoOpenCycle),
            "unknown",
            SafCode::SignatureFailed,
        ),
    ] {
        let (failure, told_code) = told(&refusal);
        assert_eq!(failure.situation, name, "{refusal:?}");
        assert_eq!(told_code, code, "{refusal:?}");
    }
}

#[test]
fn nothing_pending_is_a_situation_of_the_window_alone() {
    assert_eq!(
        Failure::from(ConsentError::NothingPending).situation,
        "siteErrandNotLive"
    );
}
