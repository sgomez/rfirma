//! Consentimiento de selección de certificados: sede, lote remoto y lote local.

use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::signing::domain::bridge::Format;
use crate::site::domain::batch::LocalSingleSign;
use crate::site::domain::protocol::{
    BatchRequest, SelectCertificate, SiteFilter, StickyCertificate,
};

use super::super::outcome::{
    BatchConsent, ErrandStep, LocalBatchConsent, LocalBatchItem, SiteOutcome,
};
use super::super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::super::request::LocalBatchAsk;
use super::super::state::LiveErrand;
use crate::site::application::batch;
use crate::site::application::filtering;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{Certificates, FilterEngine};

/// Prepara el paso de consentimiento para una selección de certificados de sede.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> ErrandStep {
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.is_headless(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    if request.is_headless() {
        if let Some(only) = the_only_one_among(&accepted) {
            return answering(live, SiteOutcome::Certificate(only));
        }
    }

    let (rows, _) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    ErrandStep::AskingForConsent {
        certificates: rows,
        filter: request.filter().clone(),
        sticky: request.sticky().is_sticky(),
    }
}

/// Prepara el consentimiento del lote remoto: los certificados cribados, cuántas firmas lleva, y el preseleccionado.
pub fn consent_to_the_batch<E: FilterEngine>(
    engine: &E,
    request: BatchRequest,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> ErrandStep {
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.is_headless(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let already_chosen = stuck.or_else(|| {
        request
            .is_headless()
            .then(|| the_only_row_among(&rows))
            .flatten()
    });

    ErrandStep::AskingToSignTheBatch(Box::new(BatchConsent {
        signs: batch::how_many(&request),
        request,
        certificates: rows,
        already_chosen,
    }))
}

/// Prepara el consentimiento del lote local: los certificados cribados del lote remoto, y el
/// resumen de qué es y qué se le hace a cada elemento (`LocalBatchSigner`, 1.9.2).
pub fn consent_to_the_local_batch<E: FilterEngine>(
    engine: &E,
    ask: LocalBatchAsk,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> ErrandStep {
    let LocalBatchAsk { request, batch } = ask;
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.is_headless(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let already_chosen = stuck.or_else(|| {
        request
            .is_headless()
            .then(|| the_only_row_among(&rows))
            .flatten()
    });

    ErrandStep::AskingToSignTheLocalBatch(Box::new(LocalBatchConsent {
        items: batch.signs().iter().map(summary_of).collect(),
        request,
        batch,
        certificates: rows,
        already_chosen,
    }))
}

/// Qué es y qué se le hace a un elemento del lote, sin su ruta ni su contenido.
fn summary_of(sign: &LocalSingleSign) -> LocalBatchItem {
    LocalBatchItem {
        id: sign.id().to_owned(),
        format: Format::from(sign.effective_format()),
        round: sign.round(),
    }
}

pub(super) fn what_the_site_accepts<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    sticky: StickyCertificate,
    headless: bool,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> Result<Vec<TokenCertificate>, ErrandStep> {
    if sticky.resets() {
        live.unstick();
    }

    if ours.is_empty() {
        return Err(no_certificate_at_all());
    }

    let owned = ours.len();
    let accepted = filtering::keep_what_the_site_accepts(engine, filter, ours, certificates)
        .map_err(|error| {
            answering(
                live,
                SiteOutcome::Refused(SiteRefusal::CouldNotFilter(error)),
            )
        })?;

    if accepted.is_empty()
        || (headless
            && accepted
                .iter()
                .all(|certificate| !certificate.status().is_usable()))
    {
        return Err(no_certificate_the_site_accepts(live, owned));
    }

    Ok(accepted)
}

/// El único certificado utilizable de la lista, que `headless` acepta sin preguntar
/// (`CertFilterManager.isMandatoryCertificate`, 1.9.2).
pub(super) fn the_only_row_among(rows: &[ListedCertificate]) -> Option<String> {
    let mut usable = rows.iter().filter(|row| row.status.is_usable());
    let only = usable.next()?;
    usable.next().is_none().then(|| only.id.clone())
}

fn the_only_one_among(accepted: &[TokenCertificate]) -> Option<Vec<u8>> {
    let mut usable = accepted
        .iter()
        .filter(|certificate| certificate.status().is_usable());
    let only = usable.next()?;
    usable.next().is_none().then(|| only.der().to_vec())
}

/// Las filas de los aceptados, con la fijada en la sesión como única preseleccionada si `sticky` la encuentra, y su asa.
pub(super) fn rows_preselecting_the_stuck(
    accepted: Vec<TokenCertificate>,
    sticky: StickyCertificate,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> (Vec<ListedCertificate>, Option<String>) {
    let stuck_at = sticky
        .is_sticky()
        .then(|| live.the_stuck())
        .flatten()
        .and_then(|stuck| {
            accepted.iter().position(|certificate| {
                stuck.is_the_same_as(certificate.reference()) && certificate.status().is_usable()
            })
        });
    let mut rows = certificates.rows_of(accepted);
    let Some(stuck_at) = stuck_at else {
        return (rows, None);
    };
    for (at, row) in rows.iter_mut().enumerate() {
        row.remembered = at == stuck_at;
    }
    let stuck = rows[stuck_at].id.clone();
    (rows, Some(stuck))
}
