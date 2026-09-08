//! Mesa del trámite: dependencias de ejecución y evaluación del consentimiento.

use std::path::PathBuf;
use std::sync::Arc;

use crate::documents::domain::handles;
use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::signing::domain::bridge::Format;
use crate::signing::domain::{AdmissibleDocument, ALLOW_UNREGISTERED_KEY};
use crate::site::domain::batch::LocalSingleSign;
use crate::site::domain::protocol::{
    forget_the_box, refuse_a_countersignature_outside_cades_and_xades, refuse_explicit_xades,
    visible_signature_of, AfirmaUrl, AskedAlgorithm, BatchRequest, LoadRequest, RequestedFormat,
    SaveRequest, SelectCertificate, SignAndSaveRequest, SignRequest, SignatureRound, SiteFilter,
    SiteVisibleSignature, StickyCertificate,
};

use super::outcome::{
    BatchConsent, ErrandStep, LoadingConsent, LocalBatchConsent, LocalBatchItem, SavingConsent,
    SavingHints, SigningConsent, SiteOutcome,
};
use super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::request::{LocalBatchAsk, SiteRequest};
use super::state::LiveErrand;
use crate::site::application::batch;
use crate::site::application::filtering;
use crate::site::application::policies;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{
    BatchServices, Certificates, FilterEngine, PolicyEngine, Scratch, ScratchDocuments,
    SiteSigning, TokenSigning,
};

/// Lo que el trámite pide a los vecinos, junto: los certificados, el documento de paso y la firma.
pub trait Neighbours: Certificates + ScratchDocuments + SiteSigning + TokenSigning {}

impl<N: Certificates + ScratchDocuments + SiteSigning + TokenSigning> Neighbours for N {}

/// Dependencias agrupadas necesarias para la ejecución de un trámite de sede.
pub struct ErrandDesk<'a, E: FilterEngine, P: PolicyEngine, N: Neighbours> {
    /// Motor de filtros criptográficos.
    pub engine: &'a E,
    /// Expansor de políticas de firma.
    pub policies: &'a P,
    /// Los vecinos: certificados, documento de paso y firma.
    pub neighbours: N,
    /// Directorio temporal para ficheros de paso.
    pub scratch_dir: PathBuf,
    /// Quien escribe y borra el fichero de paso.
    pub scratch: Arc<dyn Scratch + Send + Sync>,
    /// Los dos servlets del lote remoto.
    pub batch: Arc<dyn BatchServices + Send + Sync>,
}

/// Atiende la operación recibida por el canal local evaluando los certificados disponibles.
pub fn attend_operation<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    url: &AfirmaUrl,
    request: SiteRequest,
    live: &LiveErrand,
) -> ErrandStep {
    let operation = match request {
        SiteRequest::NotAttended(refusal) => {
            return answering(live, SiteOutcome::RefusedByTheProtocol(refusal))
        }
        attended => attended,
    };

    live.keep_the_request(url.clone());

    // `save`, `load` y un `signandsave` sin documento no miran certificados: se despachan
    // antes de pedirlos, o un equipo sin certificados se lleva un rechazo de token en una
    // operación que todavía no los necesita.
    if let SiteRequest::SignAndSave(request) = &operation {
        if request.document().is_none() {
            return consent_to_load_for_sign_and_save(request.clone());
        }
    }

    match operation {
        SiteRequest::Save(request) => return consent_to_save(request),
        SiteRequest::Load(request) => return consent_to_load(request),
        SiteRequest::SelectCertificate(_)
        | SiteRequest::Sign(_)
        | SiteRequest::SignAndSave(_)
        | SiteRequest::Batch(_)
        | SiteRequest::LocalBatch(_) => {}
        SiteRequest::NotAttended(_) => unreachable!("se ha despachado arriba"),
    }

    let ours = match desk.neighbours.listed() {
        Ok(ours) => ours,
        Err(error) => {
            return answering(live, SiteOutcome::Refused(SiteRefusal::Token(error)));
        }
    };

    match operation {
        SiteRequest::SelectCertificate(request) => {
            consent_for(desk.engine, &request, ours, &desk.neighbours, live)
        }
        SiteRequest::Sign(request) => consent_to_sign(desk, &request, ours, live),
        SiteRequest::SignAndSave(request) => consent_to_sign_and_save(desk, &request, ours, live),
        SiteRequest::Batch(request) => {
            consent_to_the_batch(desk.engine, request, ours, &desk.neighbours, live)
        }
        SiteRequest::LocalBatch(ask) => {
            consent_to_the_local_batch(desk.engine, *ask, ours, &desk.neighbours, live)
        }
        SiteRequest::Save(_) | SiteRequest::Load(_) | SiteRequest::NotAttended(_) => {
            unreachable!("se ha despachado arriba")
        }
    }
}

/// Prepara el paso de guardado: la orden de Tauri abrirá el diálogo del portal.
fn consent_to_save(request: SaveRequest) -> ErrandStep {
    ErrandStep::Saving(Box::new(SavingConsent {
        data: request.data().to_vec(),
        title: request.title().map(str::to_owned),
        filename: request.filename().map(str::to_owned),
        extensions: request.extensions().to_vec(),
        description: request.description().map(str::to_owned),
        starting_folder: None,
        signer_der: None,
    }))
}

/// Prepara el paso de carga: la orden de Tauri abrirá el selector del portal.
fn consent_to_load(request: LoadRequest) -> ErrandStep {
    ErrandStep::Loading(LoadingConsent {
        title: request.title().map(str::to_owned),
        extensions: request.extensions().to_vec(),
        description: request.description().map(str::to_owned),
        starting_folder: request.starting_folder().map(str::to_owned),
        multiple: request.multiple(),
        to_sign: None,
    })
}

/// Prepara el paso de carga cuando `signandsave` llega sin `dat`: el mismo selector que `load`,
/// de un solo fichero, con las pistas propias de `signandsave` y la petición pendiente de
/// documento (`ProtocolInvocationLauncherSignAndSave`, 1.9.2).
fn consent_to_load_for_sign_and_save(request: SignAndSaveRequest) -> ErrandStep {
    ErrandStep::Loading(LoadingConsent {
        title: None,
        extensions: request.load_extensions().to_vec(),
        description: request.load_description().map(str::to_owned),
        starting_folder: request.load_starting_folder().map(str::to_owned),
        multiple: false,
        to_sign: Some(Box::new(request)),
    })
}

/// Continúa `signandsave` con el documento que la persona acaba de elegir en el selector: mismo
/// veredicto de formato y mismas comprobaciones que si hubiera llegado en `dat`.
pub fn consent_to_sign_and_save_with_chosen_document<
    E: FilterEngine,
    P: PolicyEngine,
    N: Neighbours,
>(
    desk: &ErrandDesk<'_, E, P, N>,
    request: SignAndSaveRequest,
    document: Vec<u8>,
    chosen_name: Option<String>,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let request = request.with_chosen_document(document, chosen_name);

    consent_to_sign_and_save(desk, &request, ours, live)
}

/// Prepara el paso de consentimiento para una firma o cofirma de sede.
pub fn consent_to_sign<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    request: &SignRequest,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    consent_to_a_signature(
        desk,
        SignatureAsk {
            document: request.document(),
            format: request.format(),
            algorithm: request.algorithm(),
            round: request.round(),
            declared_params: request.declared_params(),
            filter: request.filter(),
        },
        None,
        ours,
        live,
    )
}

/// Prepara el paso de consentimiento para `signandsave`: lo de `sign`, con las pistas de
/// guardado que se contestarán tras la postfirma en vez de en el acto.
pub fn consent_to_sign_and_save<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    request: &SignAndSaveRequest,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let saving = SavingHints {
        filename: request.proposed_name(),
        extensions: request.extensions().to_vec(),
        description: request.description().map(str::to_owned),
        starting_folder: request.starting_folder().map(str::to_owned),
    };
    consent_to_a_signature(
        desk,
        SignatureAsk {
            document: request.document().unwrap_or_default(),
            format: request.format(),
            algorithm: request.algorithm(),
            round: request.round(),
            declared_params: request.declared_params(),
            filter: request.filter(),
        },
        Some(Box::new(saving)),
        ours,
        live,
    )
}

/// Lo que se firma, desacoplado de si vino de `sign` o de `signandsave`.
struct SignatureAsk<'a> {
    document: &'a [u8],
    format: RequestedFormat,
    algorithm: AskedAlgorithm,
    round: SignatureRound,
    declared_params: &'a [(String, String)],
    filter: &'a SiteFilter,
}

/// El cuerpo compartido de `consent_to_sign` y `consent_to_sign_and_save`.
fn consent_to_a_signature<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    ask: SignatureAsk<'_>,
    saving: Option<Box<SavingHints>>,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    if let Err(refusal) = refuse_a_countersignature_outside_cades_and_xades(ask.round, ask.format) {
        return answering(live, SiteOutcome::RefusedByTheProtocol(refusal));
    }

    if let Err(refusal) = refuse_explicit_xades(ask.format, ask.declared_params) {
        return answering(live, SiteOutcome::RefusedByTheProtocol(refusal));
    }

    let format = match Format::from(ask.format).bridged() {
        Ok(format) => format,
        Err(error) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::FormatNotBridged(error)),
            )
        }
    };

    let admitted = match AdmissibleDocument::check_for(format, ask.document) {
        Ok(admitted) => admitted,
        Err(inadmissible) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::Inadmissible(inadmissible)),
            )
        }
    };

    let mut from_the_site =
        match policies::expanded_for_the_site(desk.policies, ask.declared_params, format) {
            Ok(expanded) => expanded,
            Err(error) => {
                return answering(live, SiteOutcome::Refused(SiteRefusal::Policies(error)))
            }
        };

    let allowed_by_the_site = from_the_site
        .remove(ALLOW_UNREGISTERED_KEY)
        .map(|declared| declared.trim().eq_ignore_ascii_case("true"));
    let unregistered_signatures = admitted.has_unregistered_signatures();
    if unregistered_signatures && allowed_by_the_site == Some(false) {
        return answering(live, SiteOutcome::Cancelled);
    }

    let visible = if format == Format::Pades {
        match visible_signature_of(&from_the_site) {
            Ok(visible) => visible,
            Err(refusal) => return answering(live, SiteOutcome::RefusedByTheProtocol(refusal)),
        }
    } else {
        forget_the_box(&mut from_the_site);
        SiteVisibleSignature::Declined
    };

    let accepted = match accepted_listing(desk, ask.filter, ours, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let document = match keep_the_document(desk, live, format, ask.document) {
        Ok(document) => document,
        Err(refusal) => return answering(live, SiteOutcome::Refused(refusal)),
    };

    ErrandStep::AskingToSign(SigningConsent {
        document,
        format,
        algorithm: ask.algorithm,
        round: ask.round,
        certificates: desk.neighbours.rows_of(accepted),
        from_the_site,
        visible,
        filter: ask.filter.clone(),
        unregistered_signatures,
        saving,
    })
}

fn accepted_listing<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    filter: &SiteFilter,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> Result<Vec<TokenCertificate>, ErrandStep> {
    if ours.is_empty() {
        return Err(no_certificate_at_all());
    }

    let owned = ours.len();
    let accepted =
        filtering::keep_what_the_site_accepts(desk.engine, filter, ours).map_err(|error| {
            answering(
                live,
                SiteOutcome::Refused(SiteRefusal::CouldNotFilter(error)),
            )
        })?;

    if accepted.is_empty() {
        return Err(no_certificate_the_site_accepts(live, owned));
    }
    Ok(accepted)
}

fn keep_the_document<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
    format: Format,
    bytes: &[u8],
) -> Result<String, SiteRefusal> {
    desk.scratch
        .make_the_folder(&desk.scratch_dir)
        .map_err(SiteRefusal::ScratchFolderMissing)?;
    let path = desk
        .scratch_dir
        .join(format!("{}.{}", handles::mint(), what_arrives_in(format)));
    desk.scratch
        .write(&path, bytes)
        .map_err(SiteRefusal::ScratchUnwritable)?;
    live.keep_the_scratch(path.clone(), desk.scratch.clone());
    Ok(desk.neighbours.open_unrecorded(path))
}

/// La extensión del documento que se firma en ese formato, no la de la firma que sale.
fn what_arrives_in(format: Format) -> &'static str {
    match format {
        Format::Pades => "pdf",
        Format::Xades(_) | Format::XmlDsig(_) | Format::FacturaE => "xml",
        Format::Cades | Format::CadesAsicS | Format::Cms => "bin",
    }
}

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
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    if request.sticky().is_sticky() {
        if let Some(stuck) = the_remembered_one_among(&accepted, certificates) {
            return answering(live, SiteOutcome::Certificate(stuck));
        }
    }

    ErrandStep::AskingForConsent {
        certificates: certificates.rows_of(accepted),
        filter: request.filter().clone(),
        sticky: request.sticky().is_sticky(),
    }
}

/// Prepara el consentimiento del lote remoto: los mismos certificados cribados que una firma, con
/// cuántas firmas lleva el lote, y sin preguntar cuando `sticky` ya lo resolvió
/// (`ProtocolInvocationLauncherBatch`, 1.9.2).
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
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let rows = certificates.rows_of(accepted);
    let already_chosen = request
        .sticky()
        .is_sticky()
        .then(|| the_remembered_row_among(&rows))
        .flatten();

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
        ours,
        certificates,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let rows = certificates.rows_of(accepted);
    let already_chosen = request
        .sticky()
        .is_sticky()
        .then(|| the_remembered_row_among(&rows))
        .flatten();

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

fn what_the_site_accepts<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    sticky: StickyCertificate,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> Result<Vec<TokenCertificate>, ErrandStep> {
    if sticky.resets() {
        certificates.forget_the_remembered();
    }

    if ours.is_empty() {
        return Err(no_certificate_at_all());
    }

    let owned = ours.len();
    let accepted =
        filtering::keep_what_the_site_accepts(engine, filter, ours).map_err(|error| {
            answering(
                live,
                SiteOutcome::Refused(SiteRefusal::CouldNotFilter(error)),
            )
        })?;

    if accepted.is_empty() {
        return Err(no_certificate_the_site_accepts(live, owned));
    }

    Ok(accepted)
}

fn the_remembered_row_among(rows: &[ListedCertificate]) -> Option<String> {
    rows.iter()
        .find(|row| row.remembered && row.status.is_usable())
        .map(|row| row.id.clone())
}

fn the_remembered_one_among(
    accepted: &[TokenCertificate],
    certificates: &dyn Certificates,
) -> Option<Vec<u8>> {
    let remembered = certificates.remembered()?;
    accepted
        .iter()
        .find(|certificate| {
            remembered.is_the_same_as(certificate.reference()) && certificate.status().is_usable()
        })
        .map(|certificate| certificate.der().to_vec())
}
