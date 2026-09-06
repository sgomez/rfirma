//! Trámite de sede: atención de la operación del canal, consentimiento y entrega de respuesta.

pub mod desk;
pub mod outcome;
pub mod replies;
pub mod request;
pub mod state;

#[cfg(test)]
mod tests;

use crate::identity::domain::secret::StoreSecret;
use crate::signing::adapters::orders::{SigningOrder, VisibleFieldsOrder};
use crate::site::domain::protocol::AfirmaUrl;

use crate::signing::ports::{FilterEngine, IsolateHost, PolicyEngine};
use crate::site::application::session::{self as signing, SiteSigning};

pub use crate::site::application::session::SiteRefusal;
pub use crate::site::ports::{ChannelTransport, Inbox, ProtocolCodec, ReplyHandle, Transport};
pub use desk::{attend_operation, consent_for, consent_to_sign, ErrandDesk};
pub use outcome::{ErrandStep, Moment, NoCertificate, NoChannel, SigningConsent, SiteOutcome};
pub use replies::{
    declined, identify_with, identity_handed_over, signature_handed_over,
    the_signature_did_not_come_out,
};
pub use request::SiteRequest;
pub use state::{Errand, LiveErrand, NegotiatedCodec};

/// Atiende la operación recibida por el canal local.
pub fn attend<E: FilterEngine, P: PolicyEngine, I: IsolateHost>(
    desk: &ErrandDesk<'_, E, P, I>,
    url: AfirmaUrl,
    reply: ReplyHandle,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    live.answer_through(reply);
    dispatch(desk, url, live)
}

/// Reevalúa la petición recibida tras un cambio en los certificados disponibles.
pub fn look_again<E: FilterEngine, P: PolicyEngine, I: IsolateHost>(
    desk: &ErrandDesk<'_, E, P, I>,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let url = live.the_request()?;
    dispatch(desk, url, live)
}

fn dispatch<E: FilterEngine, P: PolicyEngine, I: IsolateHost>(
    desk: &ErrandDesk<'_, E, P, I>,
    url: AfirmaUrl,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let codec = live.codec()?;
    let step = desk::attend_operation(desk, &url, codec.decode(&url), live);

    match &step {
        ErrandStep::AskingForConsent { filter, .. } => live.remember_identity(filter.clone()),
        ErrandStep::AskingToSign(asked) => live.remember_signature(state::PendingSignature {
            document: asked.document.clone(),
            filter: asked.filter.clone(),
            from_the_site: asked.from_the_site.clone(),
            unregistered_signatures: asked.unregistered_signatures,
        }),
        ErrandStep::NoCertificate { .. } => live.forget_the_consent(),
        ErrandStep::Answering(_) => {}
    }

    if let Some(moment) = step.moment() {
        live.note(moment);
    }
    Some(step)
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
pub fn consent<E: FilterEngine, P: PolicyEngine, I: IsolateHost>(
    desk: &ErrandDesk<'_, E, P, I>,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, ConsentError> {
    if let Some(filter) = live.what_the_site_asked() {
        let outcome = identify_with(
            desk.engine,
            desk.token,
            &desk.stores,
            &filter,
            certificate,
            desk.listed,
            live,
        );
        return match outcome {
            SiteOutcome::Refused(refusal) => Err(ConsentError::Refused(refusal)),
            _ => Ok(Consented::IdentityHandedOver),
        };
    }

    let Some(pending) = live.the_signature_consented() else {
        return Err(ConsentError::NothingPending);
    };

    let order = SigningOrder {
        document: pending.document,
        certificate: certificate.to_owned(),
        placement: None,
        fields: VisibleFieldsOrder::default(),
        reason: String::new(),
        signed_at: String::new(),
        rubric: None,
        language: String::new(),
        allow_unregistered_signatures: pending.unregistered_signatures,
    };

    signing::begin_for_the_site(
        &SiteSigning {
            engine: desk.engine,
            token: desk.token,
            filter: &pending.filter,
            from_the_site: &pending.from_the_site,
        },
        &order,
        &desk.stores,
        desk.listed,
        desk.opened,
        desk.isolate,
        desk.session,
    )
    .map(Consented::SigningWith)
    .map_err(|refusal| ConsentError::Refused(told_to_the_site(live, refusal)))
}

/// Completa la fase final de la firma para la sede y entrega el resultado.
pub fn finish<E: FilterEngine, P: PolicyEngine, I: IsolateHost>(
    desk: &ErrandDesk<'_, E, P, I>,
    live: &LiveErrand,
) -> Result<(), SiteRefusal> {
    let signed = signing::finish_for_the_site(desk.isolate, desk.session)
        .map_err(|refusal| told_to_the_site(live, refusal))?;
    signature_handed_over(live, &signed);
    Ok(())
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
