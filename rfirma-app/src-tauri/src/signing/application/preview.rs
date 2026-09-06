//! Prefirma en seco: composición del PDF con sello visible sin interactuar con el token (ADR-0001, ADR-0006).

use crate::commands::Failure;
use crate::documents::application::documents;
use crate::documents::application::opened::OpenedDocuments;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::store::Store;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::adapters::orders::SigningOrder;
use crate::signing::application::cycle::{self, SigningRequest, TokenSignature};
use crate::signing::application::session::{admitted_bytes, on_the_bridge, plan_signature};
use crate::signing::domain::AdmissibleDocument;

/// Compone el PDF con el sello visible sin ejecutar la fase de firma.
pub fn compose(
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
) -> Result<Vec<u8>, Failure> {
    let document = documents::opened_document(opened, &order.document)?;
    let bytes = admitted_bytes(&document)?;
    let (config, reference, chain) = plan_signature(stores, listed, order)?;

    on_the_bridge(isolate, move |bridge| {
        let document = AdmissibleDocument::check(&bytes)?;
        let cycle = cycle::presign(
            bridge,
            SigningRequest {
                document,
                chain: &chain,
                config: &config,
                from_the_site: &crate::signing::application::cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )?;
        let seal = cycle.seal_in_transit();
        cycle.postsign(bridge, &TokenSignature::invented(), &seal)
    })
}

#[cfg(test)]
mod tests;
