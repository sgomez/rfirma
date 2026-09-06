//! La única traducción de los rechazos del trámite: a la vista de la ventana y al código de la sede (ADR-0009).

use crate::commands::Failure;
use crate::identity::adapters::failures::code_of_token;
use crate::signing::adapters::failures::{code_of_bridge, code_of_cycle, code_of_inadmissible};
use crate::site::application::errand::{ConsentError, SiteRefusal};
use crate::site::domain::channel::Situation as ChannelSituation;
use crate::site::domain::protocol::{SafCode, WireAnswer};

/// La vista para la ventana y el código para la sede de un rechazo, decididos juntos.
pub fn told(refusal: &SiteRefusal) -> (Failure, SafCode) {
    match refusal {
        SiteRefusal::Token(error) => (
            Failure::from(error.clone()),
            code_of_token(error.situation()),
        ),
        SiteRefusal::Inadmissible(refusal) => {
            (Failure::from(*refusal), code_of_inadmissible(*refusal))
        }
        SiteRefusal::Policies(error) => (Failure::from(error), code_of_bridge(error)),
        SiteRefusal::CouldNotFilter(error) => (Failure::from(error), SafCode::CannotAccessKeystore),
        SiteRefusal::NoCertificateTheSiteAccepts => (
            Failure::new(
                "certificateNotFound",
                "no queda ningun certificado que la sede acepte",
            ),
            SafCode::NoCertificatesInKeystore,
        ),
        SiteRefusal::NotUsableForTheSite(error) => {
            (Failure::from(error), SafCode::NoCertificatesInKeystore)
        }
        SiteRefusal::ScratchFolderMissing(detail) => (
            Failure::new("folderMissing", detail.clone()),
            SafCode::CannotSaveData,
        ),
        SiteRefusal::ScratchUnwritable(detail) => (
            Failure::new("unwritable", detail.clone()),
            SafCode::CannotSaveData,
        ),
        SiteRefusal::Cycle(failure) => (Failure::from(failure), code_of_cycle(failure)),
    }
}

/// Código que recibe la sede por un rechazo del trámite.
pub fn code_of(refusal: &SiteRefusal) -> SafCode {
    told(refusal).1
}

impl From<&SiteRefusal> for Failure {
    fn from(refusal: &SiteRefusal) -> Self {
        told(refusal).0
    }
}

impl From<SiteRefusal> for Failure {
    fn from(refusal: SiteRefusal) -> Self {
        Self::from(&refusal)
    }
}

impl From<ConsentError> for Failure {
    fn from(error: ConsentError) -> Self {
        match error {
            ConsentError::NothingPending => Self::new(
                "siteErrandNotLive",
                "no hay ninguna identificacion ni firma pendiente que contestar",
            ),
            ConsentError::Refused(refusal) => refusal.into(),
        }
    }
}

/// Código de protocolo de una situación del canal local.
pub fn code_of_channel(situation: ChannelSituation) -> SafCode {
    match situation {
        ChannelSituation::NoDrawnPortIsFree | ChannelSituation::NotListening => {
            SafCode::CannotOpenSocket
        }
        ChannelSituation::MaterialNotUsable => SafCode::CannotAccessSslKeystore,
    }
}

/// Construye la respuesta de cancelación voluntaria por parte del usuario.
pub fn cancelled() -> WireAnswer {
    WireAnswer::Cancelled
}

#[cfg(test)]
mod tests;
