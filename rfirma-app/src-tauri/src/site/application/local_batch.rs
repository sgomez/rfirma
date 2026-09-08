//! El lote local ya consentido: el bucle por elemento con el ciclo de sede y `stoponerror` (`LocalBatchSigner.signLocalBatch`, 1.9.2).

use std::collections::BTreeMap;

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::Format;
use crate::site::application::errand::desk::{keep_the_document, ErrandDesk, Neighbours};
use crate::site::application::errand::state::LiveErrand;
use crate::site::application::session::SiteRefusal;
use crate::site::domain::batch::{LocalBatch, LocalBatchResult, LocalSingleSign};
use crate::site::domain::protocol::AskedAlgorithm;
use crate::site::ports::{FilterEngine, PolicyEngine, SiteSigningRequest};

/// Caso de uso: firma cada elemento del lote local con el ciclo de sede, aplicando `stoponerror`.
pub fn signed_local_batch<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
    certificate: &TokenCertificate,
    secret: &str,
    batch: &LocalBatch,
) -> Result<Vec<LocalBatchResult>, SiteRefusal> {
    if batch.signs().is_empty() {
        return Err(SiteRefusal::LocalBatch(
            "el lote local no declara ninguna firma".to_owned(),
        ));
    }

    let Some(algorithm) = AskedAlgorithm::named(batch.algorithm()) else {
        return Err(SiteRefusal::LocalBatch(format!(
            "el lote local pide un algoritmo que no se reconoce: {}",
            batch.algorithm()
        )));
    };

    let mut results = Vec::with_capacity(batch.signs().len());
    let mut stopped = false;

    for sign in batch.signs() {
        if stopped {
            results.push(LocalBatchResult::skipped(sign.id()));
            continue;
        }

        match sign_one(desk, live, certificate, algorithm, secret, sign) {
            Ok(signature) => results.push(LocalBatchResult::signed(sign.id(), signature)),
            Err(refusal) => {
                results.push(LocalBatchResult::failed(sign.id(), refusal.description()));
                if batch.stops_on_error() {
                    let last = results.len() - 1;
                    for previous in &mut results[..last] {
                        previous.skip();
                    }
                    stopped = true;
                }
            }
        }
    }

    Ok(results)
}

/// El ciclo de una firma para un elemento: abre, firma con el secreto ya conocido y cierra.
fn sign_one<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
    certificate: &TokenCertificate,
    algorithm: AskedAlgorithm,
    secret: &str,
    sign: &LocalSingleSign,
) -> Result<Vec<u8>, SiteRefusal> {
    let format = Format::from(sign.effective_format())
        .bridged()
        .map_err(SiteRefusal::FormatNotBridged)?;

    let document = keep_the_document(desk, live, format, sign.document())?;
    let from_the_site: BTreeMap<String, String> = sign.extra_params().iter().cloned().collect();

    desk.neighbours.begin(SiteSigningRequest {
        document: &document,
        certificate,
        format,
        algorithm,
        operation: sign.round().into(),
        from_the_site: &from_the_site,
        allow_unregistered_signatures: false,
    })?;

    desk.neighbours.sign_on_token(secret)?;

    let signed = desk.neighbours.finish()?;
    Ok(signed.signature)
}

#[cfg(test)]
mod tests;
