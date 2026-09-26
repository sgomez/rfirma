//! Mesa del trámite: dependencias de ejecución y evaluación del consentimiento.

mod certificates;
mod confirmation;
mod scratch;

pub use certificates::{consent_for, consent_to_the_batch, consent_to_the_local_batch};
use certificates::{rows_preselecting_the_stuck, what_the_site_accepts, Preselected};
pub use confirmation::consent_to_the_confirmed_signature;
use confirmation::{
    asking_to_confirm, asks_to_check_signatures, the_previous_signatures_hold,
    SIGNING_CERTIFIED_PDF,
};
pub(in crate::site::application) use scratch::{keep_the_document, write_the_document};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::Format;
use crate::signing::domain::{
    AdmissibleDocument, Refusal, Waivers, ALLOW_SIGNING_CERTIFIED_KEY, ALLOW_UNREGISTERED_KEY,
};
use crate::site::domain::protocol::{
    forget_the_box, refuse_a_countersignature_outside_cades_and_xades,
    refuse_a_multisignature_of_an_invoice, refuse_explicit_xades, visible_signature_of, AfirmaUrl,
    AskedAlgorithm, LoadRequest, PendingSignRequest, RequestedFormat, SaveRequest,
    SignAndSaveRequest, SignRequest, SignatureRound, SiteFilter, SiteVisibleSignature,
    StickyCertificate,
};
use crate::site::domain::triphase_server::ServerFormat;

use super::outcome::{
    ErrandStep, ForTheSiteServer, LoadingConsent, PendingSignature, SavingConsent, SavingHints,
    SigningConsent, SiteOutcome,
};
use super::replies::answering;
use super::request::SiteRequest;
use super::state::LiveErrand;
use crate::site::application::policies;
use crate::site::application::session::SiteRefusal;
pub use crate::site::ports::Neighbours;
use crate::site::ports::{
    BatchServices, FilterEngine, PolicyEngine, Scratch, TriphaseServer, ValidationEngine,
};

/// Dependencias agrupadas necesarias para la ejecución de un trámite de sede.
pub struct ErrandDesk<'a, E: FilterEngine, P: PolicyEngine, N: Neighbours> {
    /// Motor de filtros criptográficos.
    pub engine: &'a E,
    /// Expansor de políticas de firma.
    pub policies: &'a P,
    /// Validador de las firmas que ya trae el documento.
    pub validation: &'a dyn ValidationEngine,
    /// Los vecinos: certificados, documento de paso y firma.
    pub neighbours: N,
    /// Directorio temporal para ficheros de paso.
    pub scratch_dir: PathBuf,
    /// Quien escribe y borra el fichero de paso.
    pub scratch: Arc<dyn Scratch + Send + Sync>,
    /// Los dos servlets del lote remoto.
    pub batch: Arc<dyn BatchServices + Send + Sync>,
    /// El servidor trifásico que la sede nombra en `serverUrl`.
    pub triphase: Arc<dyn TriphaseServer + Send + Sync>,
}

/// Atiende la operación recibida por el canal local evaluando los certificados disponibles.
pub fn attend_operation<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    url: &AfirmaUrl,
    request: SiteRequest,
    live: &LiveErrand,
) -> ErrandStep {
    let operation = match request {
        SiteRequest::NotAttended(refusal) if refusal.is_shown_before_it_is_answered() => {
            return ErrandStep::ShowingTheRefusal(refusal)
        }
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
        SiteRequest::SignWithoutDocument(request) => {
            return consent_to_load_for_a_signature(request)
        }
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
        SiteRequest::Save(_)
        | SiteRequest::Load(_)
        | SiteRequest::SignWithoutDocument(_)
        | SiteRequest::NotAttended(_) => {
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
    ErrandStep::Loading(Box::new(LoadingConsent {
        title: request.title().map(str::to_owned),
        filename: None,
        extensions: request.extensions().to_vec(),
        description: request.description().map(str::to_owned),
        starting_folder: request.starting_folder().map(str::to_owned),
        multiple: request.multiple(),
        to_sign: None,
    }))
}

/// Prepara el paso de carga cuando `sign`, `cosign` o `countersign` llegan sin `dat`: el mismo
/// selector que `load`, de un solo fichero, con las pistas de carga que declaró la sede
/// (`ProtocolInvocationLauncherSign`, 1.9.2).
fn consent_to_load_for_a_signature(request: PendingSignRequest) -> ErrandStep {
    ErrandStep::Loading(Box::new(LoadingConsent {
        title: None,
        filename: request.load_filename().map(str::to_owned),
        extensions: request.load_extensions().to_vec(),
        description: request.load_description().map(str::to_owned),
        starting_folder: request.load_starting_folder().map(str::to_owned),
        multiple: false,
        to_sign: Some(Box::new(PendingSignature::Signing(request))),
    }))
}

/// Prepara el paso de carga cuando `signandsave` llega sin `dat`: el mismo selector que `load`,
/// de un solo fichero, con las pistas propias de `signandsave` y la petición pendiente de
/// documento (`ProtocolInvocationLauncherSignAndSave`, 1.9.2).
fn consent_to_load_for_sign_and_save(request: SignAndSaveRequest) -> ErrandStep {
    ErrandStep::Loading(Box::new(LoadingConsent {
        title: None,
        filename: request.load_filename().map(str::to_owned),
        extensions: request.load_extensions().to_vec(),
        description: request.load_description().map(str::to_owned),
        starting_folder: request.load_starting_folder().map(str::to_owned),
        multiple: false,
        to_sign: Some(Box::new(PendingSignature::SigningAndSaving(request))),
    }))
}

/// Continúa `signandsave` con el documento que la persona acaba de elegir en el selector: mismo
/// veredicto de formato y mismas comprobaciones que si hubiera llegado en `dat`.
pub fn consent_to_sign_with_chosen_document<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    pending: PendingSignature,
    document: Vec<u8>,
    chosen_name: Option<String>,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    match pending {
        PendingSignature::Signing(request) => {
            consent_to_sign(desk, &request.with_chosen_document(document), ours, live)
        }
        PendingSignature::SigningAndSaving(request) => consent_to_sign_and_save(
            desk,
            &request.with_chosen_document(document, chosen_name),
            ours,
            live,
        ),
    }
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
            sticky: request.sticky(),
            headless: request.is_headless(),
            waives_the_choice: request.waives_the_choice(),
            through_the_site_server: request.through_the_site_server(),
            confirmed: BTreeMap::new(),
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
            sticky: request.sticky(),
            headless: request.is_headless(),
            waives_the_choice: request.waives_the_choice(),
            through_the_site_server: request.through_the_site_server(),
            confirmed: BTreeMap::new(),
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
    sticky: StickyCertificate,
    headless: bool,
    waives_the_choice: bool,
    through_the_site_server: Option<ServerFormat>,
    confirmed: BTreeMap<String, String>,
}

/// El cuerpo compartido de `consent_to_sign` y `consent_to_sign_and_save`.
#[expect(clippy::too_many_lines)]
fn consent_to_a_signature<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    ask: SignatureAsk<'_>,
    saving: Option<Box<SavingHints>>,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    if let Err(refusal) = refuse_a_multisignature_of_an_invoice(ask.round, ask.format) {
        return ErrandStep::ShowingTheRefusal(refusal);
    }

    if let Err(refusal) = refuse_a_countersignature_outside_cades_and_xades(ask.round, ask.format) {
        return ErrandStep::ShowingTheRefusal(refusal);
    }

    if let Err(refusal) = refuse_explicit_xades(
        ask.round,
        ask.format,
        ask.through_the_site_server,
        ask.declared_params,
    ) {
        return ErrandStep::ShowingTheRefusal(refusal);
    }

    let format = Format::from(ask.format);

    let waivers = waivers_declared_in(&ask);
    let admitted = match AdmissibleDocument::check_for(format, ask.document, waivers) {
        Ok(admitted) => admitted,
        Err(inadmissible) if ask.headless && inadmissible.awaits_the_person(waivers) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::ConfirmationNeeded(
                    inadmissible.situation().to_owned(),
                )),
            )
        }
        Err(Refusal::Certified) if Refusal::Certified.awaits_the_person(waivers) => {
            return asking_to_confirm(
                &ask,
                saving,
                ALLOW_SIGNING_CERTIFIED_KEY.to_owned(),
                SIGNING_CERTIFIED_PDF.to_owned(),
            )
        }
        Err(inadmissible) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::Inadmissible(inadmissible)),
            )
        }
    };

    let mut from_the_site = match policies::expanded_for_the_site(
        desk.policies,
        ask.declared_params,
        format,
        ask.document.len(),
    ) {
        Ok(expanded) => expanded,
        Err(error) => return answering(live, SiteOutcome::Refused(SiteRefusal::Policies(error))),
    };

    from_the_site.extend(ask.confirmed.clone());
    if let SignatureRound::Counter { target } = ask.round {
        from_the_site.insert(COUNTER_TARGET_KEY.to_owned(), target.name().to_owned());
    }

    if asks_to_check_signatures(&mut from_the_site) {
        if let Err(step) = the_previous_signatures_hold(desk, &ask, saving.clone(), format, live) {
            return step;
        }
    }

    let allowed_by_the_site = from_the_site
        .remove(ALLOW_UNREGISTERED_KEY)
        .map(|declared| declared.trim().eq_ignore_ascii_case("true"));
    let unregistered_signatures = admitted.has_unregistered_signatures();
    if unregistered_signatures && allowed_by_the_site == Some(false) {
        return answering(live, SiteOutcome::Cancelled);
    }
    if unregistered_signatures && allowed_by_the_site.is_none() && ask.headless {
        return answering(
            live,
            SiteOutcome::Refused(SiteRefusal::ConfirmationNeeded(
                UNREGISTERED_SIGNATURES.to_owned(),
            )),
        );
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

    let accepted = match what_the_site_accepts(
        desk.engine,
        ask.filter,
        ask.sticky,
        ask.waives_the_choice,
        ours,
        &desk.neighbours,
        live,
    ) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let document = match keep_the_document(desk, live, format, ask.document) {
        Ok(document) => document,
        Err(refusal) => return answering(live, SiteOutcome::Refused(refusal)),
    };

    let (certificates, stuck) =
        rows_preselecting_the_stuck(accepted, ask.sticky, &desk.neighbours, live);
    let preselected = Preselected::among(
        &certificates,
        stuck,
        ask.waives_the_choice,
        &desk.neighbours,
    )
    .unless_there_is_a_notice(unregistered_signatures);
    ErrandStep::AskingToSign(Box::new(SigningConsent {
        document,
        format,
        algorithm: ask.algorithm,
        round: ask.round,
        certificates,
        from_the_site,
        visible,
        filter: ask.filter.clone(),
        unregistered_signatures,
        headless: ask.headless,
        saving,
        already_chosen: preselected.row,
        without_asking: preselected.without_asking,
        for_the_site_server: ask.through_the_site_server.map(|format| ForTheSiteServer {
            format,
            document: ask.document.to_vec(),
        }),
    }))
}

const UNREGISTERED_SIGNATURES: &str = "pdfHasUnregisteredSignatures";
const COUNTER_TARGET_KEY: &str = "target";

fn waivers_declared_in(ask: &SignatureAsk<'_>) -> Waivers {
    let declared = Waivers::declared_in(
        ask.declared_params
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .chain(
                ask.confirmed
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            ),
    );
    if ask.headless {
        declared
    } else {
        declared.the_person_types_the_password()
    }
}
