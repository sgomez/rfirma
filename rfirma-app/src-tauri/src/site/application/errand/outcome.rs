//! Vocabulario de salida del trámite con la sede y la ventana.

use std::collections::BTreeMap;

use crate::identity::domain::certificate::ListedCertificate;
use crate::site::application::session::SiteRefusal;
use crate::site::domain::protocol::{Refusal, SignatureRound, SiteFilter, SiteVisibleSignature};

/// En qué queda la operación que llegó por el canal.
#[derive(Debug)]
pub enum ErrandStep {
    /// Momento de consentimiento de certificado para la ventana.
    AskingForConsent {
        /// Certificados aceptados por la sede, ya cribados.
        certificates: Vec<ListedCertificate>,
        /// Filtro solicitado por la sede para volver a comprobarlo (ADR-0011).
        filter: SiteFilter,
    },
    /// Momento de consentimiento de firma de documento para la ventana.
    AskingToSign(SigningConsent),
    /// Trámite sin ningún certificado con el que continuar.
    NoCertificate {
        /// Razón por la que no hay certificado.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona en su almacén.
        owned: usize,
        /// Lo que la sede ya ha recibido, si correspondía enviar algo.
        answered: Option<SiteOutcome>,
    },
    /// Respuesta directa para la sede sin requerir consentimiento.
    Answering(SiteOutcome),
}

impl ErrandStep {
    /// Momento que este paso deja delante de la persona, si deja alguno.
    pub fn moment(&self) -> Option<Moment> {
        match self {
            Self::AskingForConsent { certificates, .. } => Some(Moment::AskingForConsent {
                certificates: certificates.clone(),
            }),
            Self::AskingToSign(consent) => Some(Moment::AskingToSign {
                document: consent.document.clone(),
                round: consent.round,
                certificates: consent.certificates.clone(),
                unregistered_signatures: consent.unregistered_signatures,
            }),
            Self::NoCertificate { reason, owned, .. } => Some(Moment::NoCertificate {
                reason: *reason,
                owned: *owned,
            }),
            Self::Answering(_) => None,
        }
    }
}

/// Motivo por el que no queda ningún certificado con el que seguir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoCertificate {
    /// La persona no tiene certificados instalados.
    NotOne,
    /// La sede ha excluido todos los certificados instalados.
    TheSiteExcludedThemAll,
}

/// Datos necesarios para el consentimiento de firma.
#[derive(Debug)]
pub struct SigningConsent {
    /// Identificador del documento para la ventana (ADR-0011).
    pub document: String,
    /// Modalidad de firma solicitada.
    pub round: SignatureRound,
    /// Certificados aceptados por la sede, ya cribados.
    pub certificates: Vec<ListedCertificate>,
    /// Parámetros adicionales de la sede, ya expandidos.
    pub from_the_site: BTreeMap<String, String>,
    /// Recuadro de firma visible decidido para la petición (ADR-0019).
    pub visible: SiteVisibleSignature,
    /// Filtro de certificados solicitado por la sede.
    pub filter: SiteFilter,
    /// Si el documento contiene firmas que no se pueden interpretar.
    pub unregistered_signatures: bool,
}

/// Desenlace del trámite para la sede y para la ventana.
#[derive(Debug)]
pub enum SiteOutcome {
    /// Certificado entregado por la persona, en DER.
    Certificate(Vec<u8>),
    /// Firma producida para la sede junto con el certificado firmante.
    Signature {
        /// El DER del firmante.
        signer_der: Vec<u8>,
        /// El PDF firmado.
        signed: Vec<u8>,
    },
    /// Trámite cancelado por la persona.
    Cancelled,
    /// Rechazo con su situación, que el adaptador traduce al cable y a la ventana.
    Refused(SiteRefusal),
    /// Rechazo directo del protocolo.
    RefusedByTheProtocol(Refusal),
}

impl SiteOutcome {
    /// La situación del rechazo, si lo hay.
    pub fn refusal(&self) -> Option<&SiteRefusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            _ => None,
        }
    }
}

/// Momento en el que se encuentra el trámite para la ventana de sede.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Canal abierto esperando la petición de la sede.
    Waiting,
    /// Consentimiento de identificación con certificados cribados.
    AskingForConsent {
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
    },
    /// Consentimiento de firma de documento con certificados cribados.
    AskingToSign {
        /// Identificador del documento para la ventana.
        document: String,
        /// Modalidad de firma solicitada.
        round: SignatureRound,
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
        /// Si el documento contiene firmas que no se pueden interpretar.
        unregistered_signatures: bool,
    },
    /// Trámite sin certificados disponibles.
    NoCertificate {
        /// Razón por la que no hay certificado.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona.
        owned: usize,
    },
    /// Canal con la sede no disponible.
    NoChannel(NoChannel),
    /// Rechazo del protocolo sin canal por el que responder.
    RefusedWithoutChannel(Refusal),
}

/// Motivo por el que no hay canal abierto con la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoChannel {
    /// No se pudo abrir el canal en los puertos asignados o con el material TLS.
    ChannelNotOpened,
    /// La CA local no está registrada en ningún almacén NSS (ADR-0005).
    LocalCaMissing,
}
