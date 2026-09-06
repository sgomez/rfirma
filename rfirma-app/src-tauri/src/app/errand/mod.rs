//! Trámite de sede: atención de la operación del canal, consentimiento y entrega de respuesta.

pub mod desk;
pub mod outcome;
pub mod ports;
pub mod replies;
pub mod request;
pub mod state;

#[cfg(test)]
mod tests;

use crate::commands::orders::{SigningOrder, VisibleFieldsOrder};
use crate::commands::Failure;
use crate::pkcs11::StoreSecret;
use crate::protocol::AfirmaUrl;

use crate::app::filtering::FilterEngine;
use crate::app::policies::PolicyEngine;
use crate::app::signing::{self, SiteSigning};

pub use desk::{attend_operation, consent_for, consent_to_sign, ErrandDesk};
pub use outcome::{ErrandStep, Moment, NoCertificate, NoChannel, SigningConsent, SiteOutcome};
pub use ports::{ChannelTransport, Inbox, ProtocolCodec, ReplyHandle, Transport};
pub use replies::{
    declined, identify_with, identity_handed_over, signature_handed_over,
    the_signature_did_not_come_out,
};
pub use request::SiteRequest;
pub use state::{Errand, LiveErrand, NegotiatedCodec};

/// Atiende la operación recibida por el canal local.
pub fn attend<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    url: AfirmaUrl,
    reply: ReplyHandle,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    live.answer_through(reply);
    dispatch(desk, url, live)
}

/// Reevalúa la petición recibida tras un cambio en los certificados disponibles.
pub fn look_again<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
) -> Option<ErrandStep> {
    let url = live.the_request()?;
    dispatch(desk, url, live)
}

fn dispatch<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
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

/// Registra el consentimiento con el certificado seleccionado y avanza el trámite.
pub fn consent<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, Failure> {
    if let Some(filter) = live.what_the_site_asked() {
        let outcome = identify_with(
            desk.engine,
            &desk.stores,
            &filter,
            certificate,
            desk.listed,
            live,
        );
        return match outcome.failure() {
            Some(failure) => Err(failure.clone()),
            None => Ok(Consented::IdentityHandedOver),
        };
    }

    let Some(pending) = live.the_signature_consented() else {
        return Err(Failure::new(
            "siteErrandNotLive",
            "no hay ninguna identificacion ni firma pendiente que contestar",
        ));
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
    .map_err(|refusal| {
        let failure = refusal.failure().clone();
        the_signature_did_not_come_out(live, refusal);
        failure
    })
}

/// Completa la fase final de la firma para la sede y entrega el resultado.
pub fn finish<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    live: &LiveErrand,
) -> Result<(), Failure> {
    let signed = signing::finish_for_the_site(desk.isolate, desk.session).map_err(|refusal| {
        let failure = refusal.failure().clone();
        the_signature_did_not_come_out(live, refusal);
        failure
    })?;
    signature_handed_over(live, &signed);
    Ok(())
}

/// Cancela el trámite de sede notificando cancelación a la sede.
pub fn decline(live: &LiveErrand) -> SiteOutcome {
    declined(live)
}
