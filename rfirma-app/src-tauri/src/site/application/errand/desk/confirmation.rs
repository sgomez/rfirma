//! La confirmación que la firma de sede espera de la persona antes de seguir; no decide el consentimiento.

use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::{Format, SignatureVerdict};
use crate::site::application::session::SiteRefusal;
use crate::site::domain::protocol::SignatureRound;
use crate::site::ports::{FilterEngine, PolicyEngine};

use super::{consent_to_a_signature, ErrandDesk, Neighbours, SignatureAsk};
use crate::site::application::errand::outcome::{
    ConfirmationConsent, ErrandStep, SavingHints, SiteOutcome,
};
use crate::site::application::errand::replies::answering;
use crate::site::application::errand::state::LiveErrand;

/// `properties`: la sede pide validar las firmas que ya trae el documento antes de seguir.
const CHECK_SIGNATURES: &str = "checkSignatures";

/// El código con el que el original pregunta por un PDF certificado.
pub(super) const SIGNING_CERTIFIED_PDF: &str = "signingCertifiedPdf";

/// Si la sede pidió validar las firmas previas; la clave la interpreta el trámite y no cruza al puente.
pub(super) fn asks_to_check_signatures(from_the_site: &mut BTreeMap<String, String>) -> bool {
    from_the_site
        .remove(CHECK_SIGNATURES)
        .is_some_and(|declared| declared.trim().eq_ignore_ascii_case("true"))
}

/// El veredicto del validador del original sobre lo que el documento ya traía firmado.
pub(super) fn the_previous_signatures_hold<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    ask: &SignatureAsk<'_>,
    saving: Option<Box<SavingHints>>,
    format: Format,
    live: &LiveErrand,
) -> Result<(), ErrandStep> {
    match desk
        .validation
        .verdict_of(&STANDARD.encode(ask.document), format)
    {
        Ok(SignatureVerdict::Valid) => Ok(()),
        Ok(SignatureVerdict::Unsigned) if ask.round == SignatureRound::First => Ok(()),
        Ok(SignatureVerdict::Unsigned) => Err(answering(
            live,
            SiteOutcome::Refused(SiteRefusal::InvalidSignature("NO_SIGN".to_owned())),
        )),
        Ok(SignatureVerdict::Invalid { reason }) => Err(answering(
            live,
            SiteOutcome::Refused(SiteRefusal::InvalidSignature(reason)),
        )),
        Ok(SignatureVerdict::ConfirmationNeeded { message_code, .. }) if ask.headless => {
            Err(answering(
                live,
                SiteOutcome::Refused(SiteRefusal::ConfirmationNeeded(message_code)),
            ))
        }
        Ok(SignatureVerdict::ConfirmationNeeded {
            parameter,
            message_code,
        }) => Err(asking_to_confirm(ask, saving, parameter, message_code)),
        Err(error) => Err(answering(
            live,
            SiteOutcome::Refused(SiteRefusal::CouldNotValidate(error)),
        )),
    }
}

/// El momento en que la firma no sigue sin que la persona confirme; su «seguir» fija `parameter`.
pub(super) fn asking_to_confirm(
    ask: &SignatureAsk<'_>,
    saving: Option<Box<SavingHints>>,
    parameter: String,
    message_code: String,
) -> ErrandStep {
    ErrandStep::AskingToConfirm(Box::new(ConfirmationConsent {
        document: ask.document.to_vec(),
        requested: ask.format,
        algorithm: ask.algorithm,
        round: ask.round,
        declared: ask.declared_params.to_vec(),
        filter: ask.filter.clone(),
        sticky: ask.sticky,
        headless: ask.headless,
        waives_the_choice: ask.waives_the_choice,
        through_the_site_server: ask.through_the_site_server,
        saving,
        confirmed: ask.confirmed.clone(),
        parameter,
        message_code,
    }))
}

/// Repite la firma con la clave que la persona acaba de confirmar, y vuelve a validar.
pub fn consent_to_the_confirmed_signature<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    pending: ConfirmationConsent,
    ours: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> ErrandStep {
    let mut confirmed = pending.confirmed;
    confirmed.insert(pending.parameter, "true".to_owned());
    consent_to_a_signature(
        desk,
        SignatureAsk {
            document: &pending.document,
            format: pending.requested,
            algorithm: pending.algorithm,
            round: pending.round,
            declared_params: &pending.declared,
            filter: &pending.filter,
            sticky: pending.sticky,
            headless: pending.headless,
            waives_the_choice: pending.waives_the_choice,
            through_the_site_server: pending.through_the_site_server,
            confirmed,
        },
        pending.saving,
        ours,
        live,
    )
}
