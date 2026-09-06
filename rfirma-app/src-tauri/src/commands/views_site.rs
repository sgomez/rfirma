//! Tipos de salida que cruzan hacia la ventana de sede y sus conversiones (ADR-0011).

use serde::Serialize;

use crate::app::errand::{Moment, NoCertificate, NoChannel};
use crate::protocol::{Refusal, RefusalSituation, SignatureRound};

use super::views::CertificateView;

/// Trámite de sede tal como lo recibe su ventana.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteErrandView {
    /// Origen de la petición.
    pub origin: Option<String>,
    /// Etapa actual del trámite.
    pub stage: SiteStageView,
}

impl SiteErrandView {
    /// Estado inicial a la espera de la petición de la sede.
    pub fn waiting() -> Self {
        Self {
            origin: None,
            stage: SiteStageView::Waiting,
        }
    }

    /// Estado cuando el canal no ha podido abrirse.
    pub fn no_channel(reason: NoChannelView) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::NoChannel { reason },
        }
    }

    /// Estado de rechazo de la petición sin canal disponible.
    pub fn refused(refusal: &Refusal) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::Outcome {
                outcome: SiteOutcomeView::Refused {
                    situation: refusal.situation().into(),
                    detail: refusal.detail().to_owned(),
                },
            },
        }
    }

    /// Estado cuando no hay certificados disponibles para la operación.
    pub fn without_certificates(reason: NoCertificateView, owned: usize) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::NoCertificate { reason, owned },
        }
    }

    /// Estado de solicitud de consentimiento para identificación.
    pub fn asking_for_consent(certificates: Vec<CertificateView>) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::AskingForConsent { certificates },
        }
    }

    /// Estado de solicitud de consentimiento para firma de documento.
    pub fn asking_to_sign(
        document: &str,
        round: SignatureRound,
        certificates: &[CertificateView],
        unregistered_signatures: bool,
    ) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::AskingToSign {
                document: document.to_owned(),
                round: round.into(),
                certificates: certificates.to_vec(),
                unregistered_signatures,
            },
        }
    }
}

impl From<&Moment> for SiteErrandView {
    fn from(moment: &Moment) -> Self {
        match moment {
            Moment::Waiting => Self::waiting(),
            Moment::AskingForConsent { certificates } => {
                Self::asking_for_consent(certificates.clone())
            }
            Moment::AskingToSign {
                document,
                round,
                certificates,
                unregistered_signatures,
            } => Self::asking_to_sign(document, *round, certificates, *unregistered_signatures),
            Moment::NoCertificate { reason, owned } => {
                Self::without_certificates((*reason).into(), *owned)
            }
            Moment::NoChannel(NoChannel::ChannelNotOpened) => {
                Self::no_channel(NoChannelView::ChannelNotOpened)
            }
            Moment::NoChannel(NoChannel::LocalCaMissing) => {
                Self::no_channel(NoChannelView::LocalCaMissing)
            }
            Moment::RefusedWithoutChannel(refusal) => Self::refused(refusal),
        }
    }
}

/// Tipo de operación de firma solicitada por la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureRoundView {
    /// Firma inicial de un documento.
    Sign,
    /// Cofirma de un documento previamente firmado.
    Cosign,
}

impl From<SignatureRound> for SignatureRoundView {
    fn from(round: SignatureRound) -> Self {
        match round {
            SignatureRound::First => Self::Sign,
            SignatureRound::Again => Self::Cosign,
        }
    }
}

/// Etapa del trámite mostrada en la ventana de sede.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteStageView {
    /// En espera de la petición de la sede.
    Waiting,
    /// Solicitud de consentimiento de identificación.
    AskingForConsent {
        /// Certificados disponibles para la selección.
        certificates: Vec<CertificateView>,
    },
    /// Solicitud de consentimiento de firma.
    #[serde(rename_all = "camelCase")]
    AskingToSign {
        /// Asa del documento que manda la sede.
        document: String,
        /// Tipo de firma solicitada.
        round: SignatureRoundView,
        /// Certificados disponibles para la selección.
        certificates: Vec<CertificateView>,
        /// Si el documento incluye firmas no reconocidas.
        unregistered_signatures: bool,
    },
    /// Canal no disponible.
    NoChannel {
        /// Causa de la indisponibilidad.
        reason: NoChannelView,
    },
    /// Resultado final del trámite.
    Outcome {
        /// Desenlace del trámite.
        outcome: SiteOutcomeView,
    },
    /// Sin certificados aplicables.
    #[serde(rename_all = "camelCase")]
    NoCertificate {
        /// Causa de la ausencia de certificados.
        reason: NoCertificateView,
        /// Número de certificados en el almacén.
        owned: usize,
    },
}

/// Causa por la que no hay canal de comunicación con la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoChannelView {
    /// No se pudo abrir el puerto local.
    ChannelNotOpened,
    /// La entidad emisora local no está instalada.
    LocalCaMissing,
}

/// Desenlace del trámite mostrado en la ventana.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteOutcomeView {
    /// Petición rechazada.
    Refused {
        /// Clasificación de la situación de rechazo.
        situation: RefusalSituationView,
        /// Detalle descriptivo del rechazo.
        detail: String,
    },
}

/// Clasificación de situaciones de rechazo conocidas por la ventana.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefusalSituationView {
    /// Parámetro de páginas añadidas no admitido.
    AppendedSignaturePage,
    /// Criterio de filtrado no soportado.
    UnsupportedFilter,
    /// Versión de protocolo no compatible.
    UnsupportedProtocolVersion,
    /// Falta el formato de firma en la petición.
    MissingFormat,
    /// Ya existe otro trámite en curso.
    ErrandInFlight,
    /// Situación de rechazo no clasificada.
    Unknown,
}

impl From<RefusalSituation> for RefusalSituationView {
    fn from(situation: RefusalSituation) -> Self {
        match situation {
            RefusalSituation::AppendedSignaturePage => Self::AppendedSignaturePage,
            RefusalSituation::UnsupportedFilter => Self::UnsupportedFilter,
            RefusalSituation::UnsupportedProtocolVersion => Self::UnsupportedProtocolVersion,
            RefusalSituation::MissingFormat => Self::MissingFormat,
            RefusalSituation::ErrandInFlight => Self::ErrandInFlight,
            RefusalSituation::Unknown => Self::Unknown,
        }
    }
}

impl From<NoCertificate> for NoCertificateView {
    fn from(reason: NoCertificate) -> Self {
        match reason {
            NoCertificate::NotOne => Self::None,
            NoCertificate::TheSiteExcludedThemAll => Self::Excluded,
        }
    }
}

/// Causa por la que no hay certificado disponible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoCertificateView {
    /// No hay certificados instalados en el almacén.
    None,
    /// Ninguno de los certificados cumple los criterios de la sede.
    Excluded,
}

#[cfg(test)]
mod tests;
