//! Sesión de firma de un trámite de sede: prefirma filtrada y postfirma en memoria (ADR-0001, ADR-0016).

use std::collections::BTreeMap;

use crate::documents::application::in_hand::DocumentInHand;
use crate::documents::application::opened::OpenedDocuments;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::Token;
use crate::signing::adapters::orders::SigningOrder;
use crate::signing::application::filtering;
use crate::signing::domain::bridge::BridgeError;
use crate::signing::domain::Refusal as Inadmissible;
use crate::signing::ports::{FilterEngine, IsolateHost};
use crate::site::domain::protocol::SiteFilter;

use crate::signing::application::session::{
    admitted_bytes, config_for, on_the_bridge, open_the_cycle, take_signed_cycle, CycleFailure,
    SignedCycle, SigningSession,
};

/// Por qué el trámite no sigue, antes de traducirlo a la ventana y al cable.
#[derive(Debug)]
pub enum SiteRefusal {
    /// El token no ha dejado listar los certificados.
    Token(TokenError),
    /// El documento que manda la sede no se puede firmar.
    Inadmissible(Inadmissible),
    /// Las políticas de la sede no se han podido expandir.
    Policies(BridgeError),
    /// El filtro de la sede no se ha podido aplicar al listado.
    CouldNotFilter(filtering::FilteringError),
    /// La sede excluye todos los certificados que hay.
    NoCertificateTheSiteAccepts,
    /// El certificado elegido ya no vale para la sede.
    NotUsableForTheSite(filtering::FilteringError),
    /// La carpeta de paso del documento no se ha podido crear.
    ScratchFolderMissing(String),
    /// El documento de paso no se ha podido escribir.
    ScratchUnwritable(String),
    /// El ciclo de firma ha fallado.
    Cycle(CycleFailure),
}

impl From<CycleFailure> for SiteRefusal {
    fn from(failure: CycleFailure) -> Self {
        Self::Cycle(failure)
    }
}

/// Prefirma de un trámite de sede aplicando los filtros solicitados.
pub fn begin_for_the_site<E: FilterEngine>(
    site: &SiteSigning<'_, E>,
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<StoreSecret, SiteRefusal> {
    let document = DocumentInHand::taken(opened, &order.document).map_err(CycleFailure::from)?;
    let bytes = admitted_bytes(document.document())?;
    let found = site.token.list_across(stores).map_err(CycleFailure::from)?;
    let chosen = filtering::usable_certificate_for_the_site(
        site.engine,
        site.filter,
        &found,
        &order.certificate,
        listed,
    )
    .map_err(SiteRefusal::NotUsableForTheSite)?;
    let config = config_for(order, chosen).map_err(CycleFailure::from)?;
    let reference = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];
    Ok(open_the_cycle(
        site.token,
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

/// Contexto de firma requerido por un trámite de sede.
pub struct SiteSigning<'a, E: FilterEngine> {
    /// Motor de filtros sobre certificados.
    pub engine: &'a E,
    /// El token que lista y firma.
    pub token: &'a dyn Token,
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
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<SiteSignature, SiteRefusal> {
    let SignedCycle {
        cycle,
        signature,
        seal,
        signer_der,
        ..
    } = take_signed_cycle(session)?;

    let completed = on_the_bridge(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    Ok(SiteSignature {
        signed: completed.into_pdf(),
        signer_der,
    })
}

#[cfg(test)]
mod tests;
