//! Mesa del trámite: dependencias de ejecución y evaluación del consentimiento.

use std::path::{Path, PathBuf};

use crate::documents::application::opened::OpenedDocuments;
use crate::documents::domain::handles;
use crate::identity::adapters::pkcs11;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::store::Store;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::domain::{AdmissibleDocument, ALLOW_UNREGISTERED_KEY};
use crate::site::domain::protocol::{
    visible_signature_of, AfirmaUrl, SelectCertificate, SignRequest, SiteFilter,
};
use crate::Memory;

use super::outcome::{ErrandStep, SigningConsent, SiteOutcome};
use super::replies::{answering, no_certificate_at_all, no_certificate_the_site_accepts};
use super::request::SiteRequest;
use super::state::LiveErrand;
use crate::signing::application::filtering;
use crate::signing::application::policies;
use crate::signing::application::session::SigningSession;
use crate::signing::ports::FilterEngine;
use crate::signing::ports::PolicyEngine;
use crate::site::application::session::SiteRefusal;
use crate::Environment;

/// Dependencias agrupadas necesarias para la ejecución de un trámite de sede.
pub struct ErrandDesk<'a, E: FilterEngine, P: PolicyEngine> {
    /// Motor de filtros criptográficos.
    pub engine: &'a E,
    /// Expansor de políticas de firma.
    pub policies: &'a P,
    /// Almacenes de certificados disponibles.
    pub stores: Vec<Store>,
    /// Directorio de certificados instalados.
    pub installed_dir: &'a Path,
    /// Certificados listados en la sesión.
    pub listed: &'a ListedCertificates,
    /// Documentos abiertos en la sesión.
    pub opened: &'a OpenedDocuments,
    /// Estado persistente de memoria.
    pub memory: &'a Memory,
    /// Directorio temporal para ficheros de paso.
    pub scratch_dir: PathBuf,
    /// Aislado de GraalVM para operaciones criptográficas.
    pub isolate: &'a Isolate,
    /// Sesión activa de firma.
    pub session: &'a SigningSession,
}

impl<'a> ErrandDesk<'a, Isolate, Isolate> {
    /// Construye la mesa del trámite a partir del entorno de la aplicación.
    pub fn at(
        environment: &'a Environment,
        opened: &'a OpenedDocuments,
        isolate: &'a Isolate,
        session: &'a SigningSession,
    ) -> Self {
        Self {
            engine: isolate,
            policies: isolate,
            stores: environment.all_stores(),
            installed_dir: &environment.installed_certificates,
            listed: &environment.listed,
            opened,
            memory: &environment.memory,
            scratch_dir: std::env::temp_dir(),
            isolate,
            session,
        }
    }
}

/// Atiende la operación recibida por el canal local evaluando los certificados disponibles.
pub fn attend_operation<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
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

    let ours = match pkcs11::list_certificates_across(&desk.stores) {
        Ok(ours) => ours,
        Err(error) => {
            return answering(live, SiteOutcome::Refused(SiteRefusal::Token(error)));
        }
    };

    match operation {
        SiteRequest::SelectCertificate(request) => consent_for(
            desk.engine,
            &request,
            ours,
            desk.installed_dir,
            desk.listed,
            desk.memory,
            live,
        ),
        SiteRequest::Sign(request) => consent_to_sign(desk, &request, ours, live),
        SiteRequest::NotAttended(_) => unreachable!("se ha despachado arriba"),
    }
}

/// Prepara el paso de consentimiento para una firma o cofirma de sede.
pub fn consent_to_sign<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    request: &SignRequest,
    ours: Vec<crate::identity::adapters::pkcs11::TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let admitted = match AdmissibleDocument::check(request.document()) {
        Ok(admitted) => admitted,
        Err(inadmissible) => {
            return answering(
                live,
                SiteOutcome::Refused(SiteRefusal::Inadmissible(inadmissible)),
            )
        }
    };

    let mut from_the_site =
        match policies::expanded_for_the_site(desk.policies, request.declared_params()) {
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

    let accepted = match accepted_listing(desk, request.filter(), ours, live) {
        Ok(accepted) => accepted,
        Err(step) => return step,
    };

    let document = match keep_the_document(desk, live, request.document()) {
        Ok(document) => document,
        Err(refusal) => return answering(live, SiteOutcome::Refused(refusal)),
    };

    ErrandStep::AskingToSign(SigningConsent {
        document,
        round: request.round(),
        certificates: crate::identity::application::certificates::rows_of(
            accepted,
            desk.installed_dir,
            desk.listed,
            desk.memory,
        ),
        from_the_site,
        visible,
        filter: request.filter().clone(),
        unregistered_signatures,
    })
}

fn accepted_listing<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    filter: &SiteFilter,
    ours: Vec<crate::identity::adapters::pkcs11::TokenCertificate>,
    live: &LiveErrand,
) -> Result<Vec<crate::identity::adapters::pkcs11::TokenCertificate>, ErrandStep> {
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

fn keep_the_document<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
    bytes: &[u8],
) -> Result<String, SiteRefusal> {
    std::fs::create_dir_all(&desk.scratch_dir)
        .map_err(|error| SiteRefusal::ScratchFolderMissing(error.to_string()))?;
    let path = desk.scratch_dir.join(format!("{}.pdf", handles::mint()));
    std::fs::write(&path, bytes)
        .map_err(|error| SiteRefusal::ScratchUnwritable(error.to_string()))?;
    let _ = crate::desktop::adapters::paths::restrict_to_owner(&path);
    live.keep_the_scratch(path.clone());
    Ok(desk
        .opened
        .remember_unrecorded(crate::documents::adapters::portal::PortalDocument::opened(
            path,
        )))
}

/// Prepara el paso de consentimiento para una selección de certificados de sede.
pub fn consent_for<E: FilterEngine>(
    engine: &E,
    request: &SelectCertificate,
    ours: Vec<crate::identity::adapters::pkcs11::TokenCertificate>,
    installed_dir: &Path,
    listed: &ListedCertificates,
    memory: &Memory,
    live: &LiveErrand,
) -> ErrandStep {
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

    ErrandStep::AskingForConsent {
        certificates: crate::identity::application::certificates::rows_of(
            accepted,
            installed_dir,
            listed,
            memory,
        ),
        filter: request.filter().clone(),
    }
}
