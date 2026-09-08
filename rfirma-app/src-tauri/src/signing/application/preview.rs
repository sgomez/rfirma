//! Prefirma en seco: composición del PDF con sello visible sin interactuar con el token (ADR-0001, ADR-0006).

use crate::documents::domain::document::Document;
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::cycle::{self, SigningRequest};
use crate::signing::application::session::{
    admitted_bytes, config_for, on_the_bridge, CycleFailure,
};
use crate::signing::domain::TokenSignature;
use crate::signing::domain::{AdmissibleDocument, Format, SigningChoice};
use crate::signing::ports::{DocumentBytes, IsolateHost};

/// Compone el PDF con el sello visible sin ejecutar la fase de firma.
pub fn compose(
    files: &dyn DocumentBytes,
    document: &Document,
    chosen: &TokenCertificate,
    choice: &SigningChoice,
    isolate: &impl IsolateHost,
) -> Result<Vec<u8>, CycleFailure> {
    let bytes = admitted_bytes(files, document, Format::Pades)?;
    let config = config_for(choice, chosen)?;
    let reference = chosen.reference().clone();
    let chain = vec![chosen.der().to_vec()];

    on_the_bridge(isolate, move |bridge| {
        let document = AdmissibleDocument::check_for(Format::Pades, &bytes)?;
        let cycle = cycle::presign(
            bridge,
            SigningRequest {
                format: Format::Pades,
                document,
                chain: &chain,
                config: &config,
                from_the_site: &crate::signing::application::cycle::NOTHING_FROM_A_SITE,
                certificate: &reference,
            },
        )?;
        let seal = cycle.seal_in_transit();
        let completed = cycle.postsign(bridge, &TokenSignature::invented(), &seal)?;
        Ok(completed.into_signed_document())
    })
}

#[cfg(test)]
mod tests;
