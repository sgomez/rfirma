//! Sesión de firma de un trámite de sede: prefirma filtrada y postfirma en memoria (ADR-0001, ADR-0016).

use std::collections::BTreeMap;

use crate::commands::Failure;
use crate::documents::application::in_hand::DocumentInHand;
use crate::documents::application::opened::OpenedDocuments;
use crate::identity::adapters::pkcs11::{self, Store, StoreSecret};
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::error::TokenError;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::adapters::orders::SigningOrder;
use crate::signing::application::filtering;
use crate::signing::ports::FilterEngine;
use crate::site::application::frontier;
use crate::site::domain::protocol::{SafCode, SiteFilter};

use crate::signing::application::session::{
    admitted_bytes_with_situation, config_for, on_the_bridge_with_situation, open_the_cycle,
    take_signed_cycle, CycleFailure, SignedCycle, SigningSession,
};

/// Prefirma de un trámite de sede aplicando los filtros solicitados.
pub fn begin_for_the_site<E: FilterEngine>(
    site: &SiteSigning<'_, E>,
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, SiteRefusal> {
    let document = DocumentInHand::taken(opened, &order.document)
        .map_err(|failure| SiteRefusal::new(SafCode::CannotReadData, failure))?;
    let bytes = admitted_bytes_with_situation(document.document())?;
    let found = pkcs11::list_certificates_across(stores)?;
    let chosen = filtering::usable_certificate_for_the_site(
        site.engine,
        site.filter,
        &found,
        &order.certificate,
        listed,
    )
    .map_err(|failure| SiteRefusal::new(SafCode::NoCertificatesInKeystore, failure))?;
    let config = config_for(order, chosen)
        .map_err(|failure| SiteRefusal::new(SafCode::VisibleSignature, failure))?;
    let reference = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];
    Ok(open_the_cycle(
        document,
        bytes,
        config,
        reference,
        chain,
        site.from_the_site,
        isolate,
        session,
    )?)
}

/// Resultado de rechazo de un trámite de sede con código de protocolo y detalle local.
#[derive(Debug)]
pub struct SiteRefusal {
    code: SafCode,
    failure: Failure,
}

impl SiteRefusal {
    /// Une el código del catálogo con la situación que lo decidió.
    pub fn new(code: SafCode, failure: Failure) -> Self {
        Self { code, failure }
    }

    /// Código que se enviará a la sede.
    pub fn code(&self) -> SafCode {
        self.code
    }

    /// Situación para la ventana.
    pub fn failure(&self) -> &Failure {
        &self.failure
    }

    /// Convierte el rechazo en el fallo para la ventana.
    pub fn into_failure(self) -> Failure {
        self.failure
    }
}

impl From<CycleFailure> for SiteRefusal {
    fn from(failure: CycleFailure) -> Self {
        Self::new(frontier::code_of_cycle(&failure), Failure::from(failure))
    }
}

impl From<TokenError> for SiteRefusal {
    fn from(error: TokenError) -> Self {
        Self::new(frontier::code_of_token(error.situation()), error.into())
    }
}

/// Contexto de firma requerido por un trámite de sede.
pub struct SiteSigning<'a, E: FilterEngine> {
    /// Motor de filtros sobre certificados.
    pub engine: &'a E,
    /// Filtro de certificados declarado por la sede.
    pub filter: &'a SiteFilter,
    /// Parámetros adicionales declarados por la sede.
    pub from_the_site: &'a BTreeMap<String, String>,
}

/// Firma de un trámite de sede lista para transmitir.
pub struct SiteSignature {
    /// Bytes del PDF firmado.
    pub signed: Vec<u8>,
    /// Certificado firmante en formato DER.
    pub signer_der: Vec<u8>,
}

/// Postfirma de un trámite de sede que devuelve el resultado sin persistir en disco (ADR-0011).
pub fn finish_for_the_site(
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<SiteSignature, SiteRefusal> {
    let SignedCycle {
        cycle,
        signature,
        seal,
        signer_der,
        ..
    } = take_signed_cycle(session)
        .map_err(|failure| SiteRefusal::new(SafCode::SignatureFailed, failure))?;

    let signed = on_the_bridge_with_situation(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    Ok(SiteSignature { signed, signer_der })
}

#[cfg(test)]
mod tests;
