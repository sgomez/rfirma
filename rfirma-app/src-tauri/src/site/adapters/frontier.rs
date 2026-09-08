//! La única traducción de los rechazos del trámite: a la vista de la ventana y al código de la sede (ADR-0009).

use crate::crossing::Failure;
use crate::identity::adapters::failures::code_of_token;
use crate::signing::adapters::failures::{code_of_bridge, code_of_inadmissible};
use crate::site::application::errand::{ConsentError, SiteRefusal};
use crate::site::application::filtering::FilteringError;
use crate::site::domain::batch_error::Situation as BatchSituation;
use crate::site::domain::channel::Situation as ChannelSituation;
use crate::site::domain::protocol::{SafCode, WireAnswer};
use crate::site::domain::relay_error::Situation as RelaySituation;

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
        SiteRefusal::FormatNotBridged(error) => (Failure::from(error), code_of_bridge(error)),
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
        SiteRefusal::CannotSaveData(detail) => (
            Failure::new("unwritable", detail.clone()),
            SafCode::CannotSaveData,
        ),
        SiteRefusal::CannotLoadData(detail) => (
            Failure::new("unreadable", detail.clone()),
            SafCode::CannotLoadData,
        ),
        SiteRefusal::Signing(refusal) => (
            Failure {
                situation: refusal.situation.clone(),
                detail: refusal.detail.clone(),
                attempts_left: refusal.attempts_left,
            },
            refusal.code,
        ),
        SiteRefusal::Batch(error) => (
            Failure::new(label_of_batch(error.situation()), error.detail().to_owned()),
            code_of_batch(error.situation()),
        ),
        SiteRefusal::BatchSigningFailed(refusal) => (
            Failure {
                situation: refusal.situation.clone(),
                detail: refusal.detail.clone(),
                attempts_left: refusal.attempts_left,
            },
            SafCode::BatchSignature,
        ),
    }
}

/// Etiqueta de ventana de una situación del lote remoto, tal y como la ve `told`.
fn label_of_batch(situation: BatchSituation) -> &'static str {
    match situation {
        BatchSituation::PresignerUnreachable => "presignerUnreachable",
        BatchSituation::PostsignerUnreachable => "postsignerUnreachable",
        BatchSituation::InvalidPresignResponse => "invalidPresignResponse",
        BatchSituation::InvalidPostsignResponse => "invalidPostsignResponse",
    }
}

/// Código de protocolo de una situación del lote remoto (`ProtocolInvocationLauncherBatch`, 1.9.2).
pub fn code_of_batch(situation: BatchSituation) -> SafCode {
    match situation {
        BatchSituation::PresignerUnreachable | BatchSituation::PostsignerUnreachable => {
            SafCode::ContactBatchService
        }
        BatchSituation::InvalidPresignResponse | BatchSituation::InvalidPostsignResponse => {
            SafCode::BatchSignature
        }
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
        // El servidor intermedio siempre trae su propio rechazo ya clasificado (ver `code_of_relay`).
        ChannelSituation::Relay => SafCode::CannotOpenSocket,
    }
}

/// Código de protocolo de una situación del servidor intermedio, calcados del original.
pub fn code_of_relay(situation: RelaySituation) -> SafCode {
    match situation {
        RelaySituation::ServletUnreachable => SafCode::RecoveringData,
        RelaySituation::DecryptionFailed => SafCode::DecryptingData,
        RelaySituation::UploadRejected => SafCode::SendingResult,
    }
}

/// Construye la respuesta de cancelación voluntaria por parte del usuario.
pub fn cancelled() -> WireAnswer {
    WireAnswer::Cancelled
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
