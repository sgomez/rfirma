//! Consentimiento de selección de certificados: sede, lote remoto y lote local.

use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::signing::domain::bridge::Format;
use crate::site::domain::batch::{LocalBatch, LocalSingleSign};
use crate::site::domain::protocol::{
    AlgorithmReading, AskedAlgorithm, BatchRequest, SelectCertificate, SiteFilter,
    StickyCertificate,
};

use super::super::outcome::{
    BatchConsent, ErrandStep, LocalBatchConsent, LocalBatchItem, ReceivedDocument, SiteOutcome,
};
use super::super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::super::request::LocalBatchAsk;
use super::super::state::LiveErrand;
use crate::site::application::batch;
use crate::site::application::filtering;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{FilterEngine, Neighbours, PolicyEngine};

use super::{keep_the_document, ErrandDesk};

/// Prepara el paso de consentimiento para una selección de certificados de sede.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> ErrandStep {
    let manners = CertificateManners {
        filter: request.filter(),
        sticky: request.sticky(),
        choice_waived: request.waives_the_choice(),
        headless: request.is_headless(),
    };
    let accepted = match what_the_site_accepts(engine, &manners, ours, certificates, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };
    let not_accepted = accepted.not_accepted;
    let accepted = accepted.certificates;

    if let Some(stuck) = the_usable_stuck_among(&accepted, request.sticky(), live) {
        return answering(live, SiteOutcome::Certificate(stuck.der().to_vec()));
    }

    if request.waives_the_choice() && certificates.automatic_selection_honoured() {
        if let Some(only) = the_only_one_among(&accepted) {
            if request.sticky().is_sticky() {
                live.stick(only.reference());
            }
            return answering(live, SiteOutcome::Certificate(only.der().to_vec()));
        }
    }

    let (rows, _) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    ErrandStep::AskingForConsent {
        certificates: rows,
        not_accepted,
        filter: request.filter().clone(),
        sticky: request.sticky().is_sticky(),
    }
}

/// Prepara el consentimiento del lote remoto: los certificados cribados, cuántas firmas lleva, y el preseleccionado.
pub fn consent_to_the_batch<E: FilterEngine>(
    engine: &E,
    request: BatchRequest,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> ErrandStep {
    let manners = CertificateManners {
        filter: request.filter(),
        sticky: request.sticky(),
        choice_waived: request.waives_the_choice(),
        headless: request.is_headless(),
    };
    let accepted = match what_the_site_accepts(engine, &manners, ours, certificates, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };
    let not_accepted = accepted.not_accepted;
    let accepted = accepted.certificates;

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let preselected = Preselected::among(&rows, stuck, request.waives_the_choice(), certificates);

    let asks_for_sha1 = batch::asks_for_sha1(&request);
    let sha1_allowed = asks_for_sha1 && (certificates.sha1_allowed() || live.sha1_allowed_once());
    let sha1_to_allow = asks_for_sha1 && !sha1_allowed;

    ErrandStep::AskingToSignTheBatch(Box::new(BatchConsent {
        signs: batch::how_many(&request),
        request,
        certificates: rows,
        not_accepted,
        already_chosen: preselected.row,
        without_asking: preselected.without_asking && !sha1_to_allow,
        sha1_allowed,
        sha1_to_allow,
    }))
}

/// Prepara el consentimiento del lote local: los certificados cribados del lote remoto, y cada
/// elemento como documento de paso con qué es y qué se le hace (`LocalBatchSigner`, 1.9.2).
pub fn consent_to_the_local_batch<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    ask: LocalBatchAsk,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let (engine, certificates) = (desk.engine, desk.neighbours);
    let LocalBatchAsk { request, batch } = ask;
    let manners = CertificateManners {
        filter: request.filter(),
        sticky: request.sticky(),
        choice_waived: request.waives_the_choice(),
        headless: request.is_headless(),
    };
    let accepted = match what_the_site_accepts(engine, &manners, ours, certificates, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };
    let not_accepted = accepted.not_accepted;
    let accepted = accepted.certificates;

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let preselected = Preselected::among(&rows, stuck, request.waives_the_choice(), certificates);

    let asks_for_sha1 = batch
        .as_ref()
        .is_ok_and(|batch| AskedAlgorithm::read(batch.algorithm()) == AlgorithmReading::Sha1);
    let sha1_allowed = asks_for_sha1 && (certificates.sha1_allowed() || live.sha1_allowed_once());
    let sha1_to_allow = asks_for_sha1 && !sha1_allowed;

    let items = match the_items_kept(desk, live, batch.as_ref().ok()) {
        Ok(items) => items,
        Err(refusal) => return answering(live, SiteOutcome::Refused(refusal)),
    };

    ErrandStep::AskingToSignTheLocalBatch(Box::new(LocalBatchConsent {
        items,
        request,
        batch,
        certificates: rows,
        not_accepted,
        already_chosen: preselected.row,
        without_asking: preselected.without_asking && !sha1_to_allow,
        sha1_allowed,
        sha1_to_allow,
    }))
}

/// Cada elemento del lote leído guardado como documento de paso, en su orden; ninguno si no se leyó.
fn the_items_kept<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
    batch: Option<&LocalBatch>,
) -> Result<Vec<LocalBatchItem>, SiteRefusal> {
    batch.map_or(Ok(Vec::new()), |batch| {
        batch
            .signs()
            .iter()
            .map(|sign| kept_as_an_item(desk, live, sign))
            .collect()
    })
}

/// El elemento del lote guardado como documento de paso, con qué es y qué se le hace.
fn kept_as_an_item<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
    sign: &LocalSingleSign,
) -> Result<LocalBatchItem, SiteRefusal> {
    let handle = keep_the_document(desk, live, sign.document())?;
    Ok(LocalBatchItem {
        id: sign.id().to_owned(),
        document: ReceivedDocument::kept_as(handle, Some(sign.id().to_owned()), sign.document()),
        format: Format::from(sign.effective_format()),
        round: sign.round(),
        precalculated_hash: sign
            .extra_params()
            .iter()
            .any(|(key, _)| key == super::PRECALCULATED_HASH_KEY),
    })
}

/// Lo que una petición declara sobre cómo resolver el certificado, sin los candidatos.
pub(super) struct CertificateManners<'a> {
    pub(super) filter: &'a SiteFilter,
    pub(super) sticky: StickyCertificate,
    pub(super) choice_waived: bool,
    pub(super) headless: bool,
}

/// Lo que la sede acepta de los certificados de la persona, y cuántos deja fuera.
pub(super) struct Accepted {
    pub(super) certificates: Vec<TokenCertificate>,
    pub(super) not_accepted: usize,
}

pub(super) fn what_the_site_accepts<E: FilterEngine>(
    engine: &E,
    manners: &CertificateManners<'_>,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> Result<Accepted, ErrandStep> {
    if manners.sticky.resets() {
        live.unstick();
    }

    if ours.is_empty() {
        return Err(no_certificate_at_all());
    }

    let owned = ours.len();
    let accepted =
        filtering::keep_what_the_site_accepts(engine, manners.filter, ours, certificates).map_err(
            |error| {
                answering(
                    live,
                    SiteOutcome::Refused(SiteRefusal::CouldNotFilter(error)),
                )
            },
        )?;

    if accepted.is_empty()
        || (manners.choice_waived
            && accepted
                .iter()
                .all(|certificate| !certificate.status().is_usable()))
    {
        return Err(no_certificate_the_site_accepts(
            live,
            owned,
            manners.headless,
        ));
    }

    Ok(Accepted {
        not_accepted: owned.saturating_sub(accepted.len()),
        certificates: accepted,
    })
}

/// La fila que llega elegida a la ventana, y si la ventana consiente sola con ella.
pub(super) struct Preselected {
    pub row: Option<String>,
    pub without_asking: bool,
}

impl Preselected {
    /// La fijada en la sesión o el único vigente; sin preguntar con la fijada (ADR-0010), o con el único si es la única fila y con la preferencia (`AOKeyStoreDialog.show`, 1.9.2).
    pub fn among(
        rows: &[ListedCertificate],
        stuck: Option<String>,
        choice_waived: bool,
        certificates: &dyn Neighbours,
    ) -> Self {
        let only = choice_waived.then(|| the_only_row_among(rows)).flatten();
        let without_asking = stuck.is_some()
            || (only.is_some() && rows.len() == 1 && certificates.automatic_selection_honoured());
        Self {
            row: stuck.or(only),
            without_asking,
        }
    }

    /// La misma preselección, que ya no consiente sola si el consentimiento tiene un aviso que enseñar.
    pub fn unless_there_is_a_notice(self, notice: bool) -> Self {
        Self {
            without_asking: self.without_asking && !notice,
            ..self
        }
    }
}

fn the_only_row_among(rows: &[ListedCertificate]) -> Option<String> {
    let mut usable = rows.iter().filter(|row| row.status.is_usable());
    let only = usable.next()?;
    usable.next().is_none().then(|| only.id.clone())
}

fn the_only_one_among(accepted: &[TokenCertificate]) -> Option<&TokenCertificate> {
    match accepted {
        [only] if only.status().is_usable() => Some(only),
        _ => None,
    }
}

/// Las filas de los aceptados, con la fijada en la sesión como única preseleccionada si `sticky` la encuentra, y su asa.
pub(super) fn rows_preselecting_the_stuck(
    accepted: Vec<TokenCertificate>,
    sticky: StickyCertificate,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> (Vec<ListedCertificate>, Option<String>) {
    let Some(stuck) = the_usable_stuck_among(&accepted, sticky, live) else {
        return (certificates.rows_of(accepted), None);
    };
    let mut rows = certificates.rows_of(accepted.clone());
    let stuck_at = rows.iter().position(|row| {
        certificates
            .usable(&accepted, &row.id)
            .is_ok_and(|copy| copy.is_a_copy_of(&stuck))
    });
    let Some(stuck_at) = stuck_at else {
        return (rows, None);
    };
    for (at, row) in rows.iter_mut().enumerate() {
        row.remembered = at == stuck_at;
    }
    let stuck = rows[stuck_at].id.clone();
    (rows, Some(stuck))
}

fn the_usable_stuck_among(
    accepted: &[TokenCertificate],
    sticky: StickyCertificate,
    live: &LiveErrand,
) -> Option<TokenCertificate> {
    let stuck = sticky.is_sticky().then(|| live.the_stuck()).flatten()?;
    accepted
        .iter()
        .find(|certificate| {
            stuck.is_the_same_as(certificate.reference()) && certificate.status().is_usable()
        })
        .cloned()
}
