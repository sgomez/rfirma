//! Mesa del trámite: dependencias de ejecución y evaluación del consentimiento.

use std::path::PathBuf;
use std::sync::Arc;

use crate::documents::domain::handles;
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::{AdmissibleDocument, ALLOW_UNREGISTERED_KEY};
use crate::site::domain::protocol::{
    visible_signature_of, AfirmaUrl, LoadRequest, SaveRequest, SelectCertificate,
    SignAndSaveRequest, SignRequest, SignatureRound, SiteFilter,
};

use super::outcome::{
    ErrandStep, LoadingConsent, SavingConsent, SavingHints, SigningConsent, SiteOutcome,
};
use super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::request::SiteRequest;
use super::state::LiveErrand;
use crate::site::application::filtering;
use crate::site::application::policies;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{
    Certificates, FilterEngine, PolicyEngine, Scratch, ScratchDocuments, SiteSigning,
};

/// Lo que el trámite pide a los vecinos, junto: los certificados, el documento de paso y la firma.
pub trait Neighbours: Certificates + ScratchDocuments + SiteSigning {}

impl<N: Certificates + ScratchDocuments + SiteSigning> Neighbours for N {}

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
        SiteRequest::SelectCertificate(_) | SiteRequest::Sign(_) | SiteRequest::SignAndSave(_) => {}
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
    let request = match request.with_chosen_document(document, chosen_name) {
        Ok(request) => request,
        Err(refusal) => return answering(live, SiteOutcome::RefusedByTheProtocol(refusal)),
    };

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
    let admitted = match AdmissibleDocument::check(ask.document) {
        Ok(admitted) => admitted,
        Err(inadmissible) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::Inadmissible(inadmissible)),
            )
        }
    };

    let mut from_the_site =
        match policies::expanded_for_the_site(desk.policies, ask.declared_params) {
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

    let visible = match visible_signature_of(&from_the_site) {
        Ok(visible) => visible,
        Err(refusal) => return answering(live, SiteOutcome::RefusedByTheProtocol(refusal)),
    };

    let accepted = match accepted_listing(desk, ask.filter, ours, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let document = match keep_the_document(desk, live, ask.document) {
        Ok(document) => document,
        Err(refusal) => return answering(live, SiteOutcome::Refused(refusal)),
    };

    ErrandStep::AskingToSign(SigningConsent {
        document,
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
    bytes: &[u8],
) -> Result<String, SiteRefusal> {
    desk.scratch
        .make_the_folder(&desk.scratch_dir)
        .map_err(SiteRefusal::ScratchFolderMissing)?;
    let path = desk.scratch_dir.join(format!("{}.pdf", handles::mint()));
    desk.scratch
        .write(&path, bytes)
        .map_err(SiteRefusal::ScratchUnwritable)?;
    live.keep_the_scratch(path.clone(), desk.scratch.clone());
    Ok(desk.neighbours.open_unrecorded(path))
}

/// Prepara el paso de consentimiento para una selección de certificados de sede.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<TokenCertificate>,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> ErrandStep {
    if request.sticky().resets() {
        certificates.forget_the_remembered();
    }

    if ours.is_empty() {
        return no_certificate_at_all();
    }

    let owned = ours.len();
    let accepted = match filtering::keep_what_the_site_accepts(engine, request.filter(), ours) {
        Ok(accepted) => accepted,
        Err(error) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::CouldNotFilter(error)),
            )
        }
    };

    if accepted.is_empty() {
        return no_certificate_the_site_accepts(live, owned);
    }

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
