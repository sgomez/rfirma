//! Trámite de sede: atención de la operación del canal, consentimiento y entrega de respuesta.

pub mod desk;
pub mod outcome;
pub mod replies;
pub mod request;
pub mod state;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::secret::StoreSecret;
use crate::site::domain::batch::build_local_result;
use crate::site::domain::protocol::{AfirmaUrl, SiteFilter};

use crate::site::application::batch;
use crate::site::application::local_batch;
use crate::site::application::session::{self as signing, SiteTerms};
use crate::site::ports::{FilterEngine, PolicyEngine};

pub use crate::site::application::session::SiteRefusal;
pub use crate::site::ports::{ChannelTransport, Inbox, ReplyHandle, Transport};
pub use desk::{
    attend_operation, consent_for, consent_to_sign, consent_to_sign_and_save,
    consent_to_sign_and_save_with_chosen_document, consent_to_the_batch,
    consent_to_the_local_batch, ErrandDesk, Neighbours,
};
pub use outcome::{
    BatchConsent, ErrandStep, LoadCompletion, LoadingConsent, LocalBatchConsent, LocalBatchItem,
    Moment, NoCertificate, NoChannel, ProtocolCodec, SavingConsent, SigningConsent, SiteOutcome,
};
pub use replies::{
    batch_handed_over, declined, identify_with, identity_handed_over, loaded, saved,
    signature_handed_over, the_signature_did_not_come_out,
};
pub use request::{LocalBatchAsk, SiteRequest};
pub use state::{Errand, LiveErrand, NegotiatedCodec};

/// Atiende la operación recibida por el canal local.
pub fn attend<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    url: AfirmaUrl,
    reply: ReplyHandle,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    live.answer_through(reply);
    dispatch(desk, url, live)
}

/// Reevalúa la petición recibida tras un cambio en los certificados disponibles.
pub fn look_again<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let url = live.the_request()?;
    dispatch(desk, url, live)
}

fn dispatch<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    url: AfirmaUrl,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let codec = live.codec()?;
    let step = desk::attend_operation(desk, &url, codec.decode(&url), live);
    Some(remembered(live, step))
}

/// Registra en la memoria del trámite lo que el paso deja pendiente, y publica su momento.
fn remembered(live: &LiveErrand, step: ErrandStep) -> ErrandStep {
    match &step {
        ErrandStep::AskingForConsent { filter, sticky, .. } => {
            live.remember_identity(filter.clone(), *sticky)
        }
        ErrandStep::AskingToSign(asked) => live.remember_signature(state::PendingSignature {
            document: asked.document.clone(),
            filter: asked.filter.clone(),
            format: asked.format,
            algorithm: asked.algorithm,
            operation: asked.round.into(),
            from_the_site: asked.from_the_site.clone(),
            unregistered_signatures: asked.unregistered_signatures,
            saving: asked.saving.clone(),
        }),
        ErrandStep::AskingToSignTheBatch(consent) => live.remember_the_batch(state::PendingBatch {
            request: consent.request.clone(),
            chosen: None,
        }),
        ErrandStep::AskingToSignTheLocalBatch(consent) => {
            live.remember_the_local_batch(state::PendingLocalBatch {
                request: consent.request.clone(),
                batch: consent.batch.clone(),
                chosen: None,
            })
        }
        ErrandStep::Saving(consent) => live.remember_saving((**consent).clone()),
        ErrandStep::Loading(consent) => live.remember_loading(consent.clone()),
        ErrandStep::NoCertificate { .. } => live.forget_the_consent(),
        ErrandStep::Answering(_) => {}
    }

    if let Some(moment) = step.moment() {
        live.note(moment);
    }
    step
}

/// Resultado del consentimiento de la persona usuaria.
#[derive(Debug)]
pub enum Consented {
    /// Identificación entregada a la sede.
    IdentityHandedOver,
    /// Firma iniciada requiriendo secreto al almacén.
    SigningWith(StoreSecret),
}

/// Por qué la ventana no obtiene lo que pidió del trámite.
#[derive(Debug)]
pub enum ConsentError {
    /// No hay ninguna identificación ni firma pendiente que contestar.
    NothingPending,
    /// El trámite se ha rechazado, y la sede ya lo sabe.
    Refused(SiteRefusal),
}

/// Registra el consentimiento con el certificado seleccionado y avanza el trámite.
pub fn consent<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, ConsentError> {
    if let Some((filter, sticky)) = live.what_the_site_asked() {
        let outcome = identify_with(
            desk.engine,
            &desk.neighbours,
            &filter,
            sticky,
            certificate,
            live,
        );
        return match outcome {
            SiteOutcome::Refused(refusal) => Err(ConsentError::Refused(refusal)),
            _ => Ok(Consented::IdentityHandedOver),
        };
    }

    if let Some(pending) = live.the_batch_pending() {
        return the_batch_consented(desk, &pending.request, certificate, live);
    }

    if let Some(pending) = live.the_local_batch_pending() {
        return the_local_batch_consented(desk, pending, certificate, live);
    }

    let Some(pending) = live.the_signature_consented() else {
        return Err(ConsentError::NothingPending);
    };

    signing::begin_for_the_site(
        &SiteTerms {
            engine: desk.engine,
            filter: &pending.filter,
            format: pending.format,
            algorithm: pending.algorithm,
            operation: pending.operation,
            from_the_site: &pending.from_the_site,
            allow_unregistered_signatures: pending.unregistered_signatures,
        },
        &pending.document,
        certificate,
        &desk.neighbours,
        &desk.neighbours,
    )
    .map(Consented::SigningWith)
    .map_err(|refusal| ConsentError::Refused(told_to_the_site(live, refusal)))
}

/// El certificado que la persona eligió y el secreto de su almacén, abierto una sola vez.
fn the_chosen_and_its_secret<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    filter: &SiteFilter,
    sticky: bool,
    certificate: &str,
    live: &LiveErrand,
) -> Result<(TokenCertificate, StoreSecret), ConsentError> {
    let refused =
        |live: &LiveErrand, refusal| ConsentError::Refused(told_to_the_site(live, refusal));

    let found = desk
        .neighbours
        .listed()
        .map_err(|error| refused(live, SiteRefusal::Token(error)))?;
    let chosen = crate::site::application::filtering::usable_certificate_for_the_site(
        desk.engine,
        filter,
        &found,
        certificate,
        &desk.neighbours,
    )
    .map_err(|error| refused(live, SiteRefusal::NotUsableForTheSite(error)))?;

    let secret = desk
        .neighbours
        .secret_of(chosen)
        .map_err(|refusal| refused(live, SiteRefusal::BatchSigningFailed(refusal)))?;

    if sticky {
        desk.neighbours.remember(chosen.reference());
    }

    Ok((chosen.clone(), secret))
}

/// Abre el secreto una sola vez para todas las firmas del lote remoto y apunta el consentido.
fn the_batch_consented<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    request: &crate::site::domain::protocol::BatchRequest,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, ConsentError> {
    let (chosen, secret) = the_chosen_and_its_secret(
        desk,
        request.filter(),
        request.sticky().is_sticky(),
        certificate,
        live,
    )?;

    live.remember_the_batch(state::PendingBatch {
        request: request.clone(),
        chosen: Some(chosen),
    });
    Ok(Consented::SigningWith(secret))
}

/// Abre el secreto una sola vez para todas las firmas del lote local y apunta el consentido.
fn the_local_batch_consented<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    pending: state::PendingLocalBatch,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, ConsentError> {
    let (chosen, secret) = the_chosen_and_its_secret(
        desk,
        pending.request.filter(),
        pending.request.sticky().is_sticky(),
        certificate,
        live,
    )?;

    live.remember_the_local_batch(state::PendingLocalBatch {
        chosen: Some(chosen),
        ..pending
    });
    Ok(Consented::SigningWith(secret))
}

/// Completa el lote remoto con el secreto ya tecleado: prefirma, `PK1` y postfirma.
pub fn finish_the_batch<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    secret: &str,
    live: &LiveErrand,
) -> Result<(), ConsentError> {
    let pending = live
        .the_batch_pending()
        .ok_or(ConsentError::NothingPending)?;
    let chosen = pending.chosen.ok_or(ConsentError::NothingPending)?;
    let request = pending.request;

    let result = batch::signed_batch(
        &batch::BatchRun {
            services: desk.batch.as_ref(),
            token: &desk.neighbours,
            certificate: &chosen,
            secret,
        },
        &request,
    )
    .map_err(|refusal| ConsentError::Refused(told_to_the_site(live, refusal)))?;

    let signer_der = request.needcert().then(|| chosen.der().to_vec());
    batch_handed_over(live, result, signer_der);
    Ok(())
}

/// Cierra el lote local con el secreto que se tecleó una sola vez: firma cada elemento por el
/// ciclo de sede, aplicando `stoponerror`.
pub fn finish_the_local_batch<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    secret: &str,
    live: &LiveErrand,
) -> Result<(), ConsentError> {
    let pending = live
        .the_local_batch_pending()
        .ok_or(ConsentError::NothingPending)?;
    let chosen = pending.chosen.clone().ok_or(ConsentError::NothingPending)?;

    let results = local_batch::signed_local_batch(desk, &chosen, secret, &pending.batch)
        .map_err(|refusal| ConsentError::Refused(told_to_the_site(live, refusal)))?;

    let signer_der = pending.request.needcert().then(|| chosen.der().to_vec());
    batch_handed_over(live, build_local_result(&results), signer_der);
    Ok(())
}

/// Completa la fase final de la firma para la sede y entrega el resultado, o el paso de
/// guardado si la firma venía de `signandsave`.
pub fn finish<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    live: &LiveErrand,
) -> Result<Option<ErrandStep>, SiteRefusal> {
    let saving = live
        .the_signature_consented()
        .and_then(|pending| pending.saving);
    let signed = signing::finish_for_the_site(&desk.neighbours)
        .map_err(|refusal| told_to_the_site(live, refusal))?;

    Ok(match saving {
        None => {
            signature_handed_over(live, &signed);
            None
        }
        Some(hints) => Some(remembered(
            live,
            ErrandStep::Saving(Box::new((*hints).into_consent(&signed))),
        )),
    })
}

/// Completa el selector abierto por la orden de Tauri con lo que la persona eligió: si el
/// selector esperaba un documento para `signandsave`, lo lee y continúa el trámite; si era un
/// `load` corriente, entrega lo elegido a la sede.
pub fn document_chosen<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    chosen: &[(String, PathBuf)],
    live: &LiveErrand,
) -> LoadCompletion {
    let to_sign = live
        .the_loading_pending()
        .and_then(|pending| pending.to_sign);
    let Some(request) = to_sign else {
        return LoadCompletion::Delivered(loaded(desk.scratch.as_ref(), chosen, live));
    };

    let Some((name, path)) = chosen.first() else {
        return LoadCompletion::Delivered(replies::declined(live));
    };
    let chosen_name = Some(name.clone());
    let document = match desk.scratch.read(path) {
        Ok(document) => document,
        Err(detail) => {
            return LoadCompletion::Delivered(replies::over(
                live,
                SiteOutcome::Refused(SiteRefusal::CannotLoadData(detail)),
            ))
        }
    };

    let ours = match desk.neighbours.listed() {
        Ok(ours) => ours,
        Err(error) => {
            return LoadCompletion::Delivered(replies::over(
                live,
                SiteOutcome::Refused(SiteRefusal::Token(error)),
            ))
        }
    };

    LoadCompletion::Continues(remembered(
        live,
        desk::consent_to_sign_and_save_with_chosen_document(
            desk,
            *request,
            document,
            chosen_name,
            ours,
            live,
        ),
    ))
}

fn told_to_the_site(live: &LiveErrand, refusal: SiteRefusal) -> SiteRefusal {
    match the_signature_did_not_come_out(live, refusal) {
        SiteOutcome::Refused(refusal) => refusal,
        answered => unreachable!("una firma que no sale es siempre un rechazo: {answered:?}"),
    }
}

/// Cancela el trámite de sede notificando cancelación a la sede.
pub fn decline(live: &LiveErrand) -> SiteOutcome {
    declined(live)
}
