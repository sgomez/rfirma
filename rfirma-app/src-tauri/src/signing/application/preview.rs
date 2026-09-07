//! Prefirma en seco: composición del PDF con sello visible sin interactuar con el token (ADR-0001, ADR-0006).

use crate::documents::domain::portal::PortalDocument;
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::cycle::{self, SigningRequest};
use crate::signing::application::session::{
    admitted_bytes, config_for, on_the_bridge, CycleFailure,
};
use crate::signing::domain::TokenSignature;
use crate::signing::domain::{AdmissibleDocument, SigningChoice};
use crate::signing::ports::IsolateHost;

/// Compone el PDF con el sello visible sin ejecutar la fase de firma.
pub fn compose(
    document: &PortalDocument,
    chosen: &TokenCertificate,
    choice: &SigningChoice,
    isolate: &impl IsolateHost,
) -> Result<Vec<u8>, CycleFailure> {
    let bytes = admitted_bytes(document)?;
    let config = config_for(choice, chosen)?;
    let reference = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];

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
        let completed = cycle.postsign(bridge, &TokenSignature::invented(), &seal)?;
        Ok(completed.into_pdf())
    })
}

#[cfg(test)]
mod tests;
