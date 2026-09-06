//! Prefirma en seco: composición del PDF con sello visible sin interactuar con el token (ADR-0001, ADR-0006).

use crate::app::cycle::{self, SigningRequest, TokenSignature};
use crate::app::documents;
use crate::app::signing::{admitted_bytes, on_the_bridge, plan_signature};
use crate::commands::orders::SigningOrder;
use crate::commands::views::Failure;
use crate::isolate::Isolate;
use crate::memory::{ListedCertificates, OpenedDocuments};
use crate::pkcs11::Store;
use crate::signing::AdmissibleDocument;

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
                from_the_site: &crate::app::cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )?;
        let seal = cycle.seal_in_transit();
        cycle.postsign(bridge, &TokenSignature::invented(), &seal)
    })
}

#[cfg(test)]
mod tests;
