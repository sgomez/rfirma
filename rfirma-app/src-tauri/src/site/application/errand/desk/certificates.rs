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
use crate::site::ports::{FilterEngine, Neighbours};

/// Prepara el paso de consentimiento para una selección de certificados de sede.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> ErrandStep {
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.waives_the_choice(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

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
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.waives_the_choice(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let preselected = Preselected::among(&rows, stuck, request.waives_the_choice(), certificates);

    ErrandStep::AskingToSignTheBatch(Box::new(BatchConsent {
        signs: batch::how_many(&request),
        request,
        certificates: rows,
        already_chosen: preselected.row,
        without_asking: preselected.without_asking,
    }))
}

/// Prepara el consentimiento del lote local: los certificados cribados del lote remoto, y el
/// resumen de qué es y qué se le hace a cada elemento (`LocalBatchSigner`, 1.9.2).
pub fn consent_to_the_local_batch<E: FilterEngine>(
    engine: &E,
    ask: LocalBatchAsk,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
    live: &LiveErrand,
) -> ErrandStep {
    let LocalBatchAsk { request, batch } = ask;
    let accepted = match what_the_site_accepts(
        engine,
        request.filter(),
        request.sticky(),
        request.waives_the_choice(),
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let (rows, stuck) = rows_preselecting_the_stuck(accepted, request.sticky(), certificates, live);
    let preselected = Preselected::among(&rows, stuck, request.waives_the_choice(), certificates);

    ErrandStep::AskingToSignTheLocalBatch(Box::new(LocalBatchConsent {
        items: batch
            .as_ref()
            .map(|batch| batch.signs().iter().map(summary_of).collect())
            .unwrap_or_default(),
        request,
        batch,
        certificates: rows,
        already_chosen: preselected.row,
        without_asking: preselected.without_asking,
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
    choice_waived: bool,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Neighbours,
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
        || (choice_waived
            && accepted
                .iter()
                .all(|certificate| !certificate.status().is_usable()))
    {
        return Err(no_certificate_the_site_accepts(live, owned));
    }

    Ok(accepted)
}

/// La fila que llega elegida a la ventana, y si la ventana consiente sola con ella.
pub(super) struct Preselected {
    pub row: Option<String>,
    pub without_asking: bool,
}

impl Preselected {
    /// La fijada en la sesión o el único vigente; sin preguntar, solo si es la única fila y con la preferencia (`AOKeyStoreDialog.show`, 1.9.2).
    pub fn among(
        rows: &[ListedCertificate],
        stuck: Option<String>,
        choice_waived: bool,
        certificates: &dyn Neighbours,
    ) -> Self {
        let only = choice_waived.then(|| the_only_row_among(rows)).flatten();
        let without_asking =
            only.is_some() && rows.len() == 1 && certificates.automatic_selection_honoured();
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
