//! El desenlace que cruza a la ventana de sede: el rechazo clasificado y la falta de certificado.

use serde::Serialize;

use crate::crossing::crossing;
use crate::site::application::errand::NoCertificate;
use crate::site::domain::batch_error::Situation as BatchSituation;
use crate::site::domain::protocol::RefusalSituation;

crossing! {
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
}

crossing! {
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
        /// La sede nombra un almacén de certificados que rFirma no abre.
        UnsupportedKeyStore,
        /// Ya existe otro trámite en curso.
        ErrandInFlight,
        /// Otra aplicación ocupa todos los puertos que ofrece la sede.
        PortsTaken,
        /// La sede pide la XAdES explícita, que rFirma no hace.
        ExplicitXades,
        /// La sede pide cofirmar o contrafirmar una factura electrónica.
        InvoiceMultisignature,
        /// La sede pide contrafirmar en un formato que no lo admite.
        UnsupportedCountersignature,
        /// El servlet de prefirma del lote remoto no respondió.
        BatchPresignerUnreachable,
        /// El servlet de postfirma del lote remoto no respondió.
        BatchPostsignerUnreachable,
        /// La respuesta de prefirma del lote remoto no tiene la forma esperada.
        BatchInvalidPresignResponse,
        /// La respuesta de postfirma del lote remoto no tiene la forma esperada.
        BatchInvalidPostsignResponse,
        /// La firma del `PRE` de una firma del lote remoto ha fallado.
        BatchSigningFailed,
        /// Situación de rechazo no clasificada.
        Unknown,
    }
}

impl From<RefusalSituation> for RefusalSituationView {
    fn from(situation: RefusalSituation) -> Self {
        match situation {
            RefusalSituation::AppendedSignaturePage => Self::AppendedSignaturePage,
            RefusalSituation::UnsupportedFilter => Self::UnsupportedFilter,
            RefusalSituation::UnsupportedProtocolVersion => Self::UnsupportedProtocolVersion,
            RefusalSituation::MissingFormat => Self::MissingFormat,
            RefusalSituation::UnsupportedKeyStore => Self::UnsupportedKeyStore,
            RefusalSituation::ErrandInFlight => Self::ErrandInFlight,
            RefusalSituation::PortsTaken => Self::PortsTaken,
            RefusalSituation::ExplicitXades => Self::ExplicitXades,
            RefusalSituation::InvoiceMultisignature => Self::InvoiceMultisignature,
            RefusalSituation::UnsupportedCountersignature => Self::UnsupportedCountersignature,
            RefusalSituation::Unknown => Self::Unknown,
        }
    }
}

impl From<BatchSituation> for RefusalSituationView {
    fn from(situation: BatchSituation) -> Self {
        match situation {
            BatchSituation::PresignerUnreachable => Self::BatchPresignerUnreachable,
            BatchSituation::PostsignerUnreachable => Self::BatchPostsignerUnreachable,
            BatchSituation::InvalidPresignResponse => Self::BatchInvalidPresignResponse,
            BatchSituation::InvalidPostsignResponse => Self::BatchInvalidPostsignResponse,
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

crossing! {
    /// Causa por la que no hay certificado disponible.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub enum NoCertificateView {
        /// No hay certificados instalados en el almacén.
        None,
        /// Ninguno de los certificados cumple los criterios de la sede.
        Excluded,
    }
}
