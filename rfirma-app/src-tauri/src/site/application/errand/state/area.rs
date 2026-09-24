//! El área de la firma visible que espera a la persona antes del consentimiento.

use crate::site::application::errand::outcome::{Moment, SigningConsent};
use crate::site::domain::protocol::{mark_the_area, IfCancelled, SiteVisibleSignature};

use super::{LiveErrand, PendingConsent};

/// El diálogo del área pendiente y el consentimiento que lo sigue.
#[derive(Clone, Debug)]
pub(in crate::site::application::errand) struct AreaToMark {
    if_cancelled: IfCancelled,
    consenting: Moment,
}

impl AreaToMark {
    /// El área que pide marcar el consentimiento, si pide alguna.
    pub(in crate::site::application::errand) fn asked_in(consent: &SigningConsent) -> Option<Self> {
        match consent.visible {
            SiteVisibleSignature::MarkedByThePerson(if_cancelled) => Some(Self {
                if_cancelled,
                consenting: consent.consenting(),
            }),
            SiteVisibleSignature::PlacedByTheSite | SiteVisibleSignature::Declined => None,
        }
    }
}

impl LiveErrand {
    /// Qué hace cancelar el diálogo del área, si hay uno pendiente.
    pub(in crate::site::application::errand) fn the_area_to_mark(&self) -> Option<IfCancelled> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Signature(pending)) => {
                pending.area.as_ref().map(|area| area.if_cancelled)
            }
            _ => None,
        }
    }

    /// Cierra el diálogo del área, con la que marcó la persona sobre la de la petición, y pasa al consentimiento.
    pub(in crate::site::application::errand) fn settle_the_area(
        &self,
        marked: Option<Vec<(String, String)>>,
    ) {
        let consenting = {
            let mut consent = crate::lock(&self.consent);
            let Some(PendingConsent::Signature(pending)) = consent.as_mut() else {
                return;
            };
            let Some(area) = pending.area.take() else {
                return;
            };
            if let Some(marked) = marked {
                mark_the_area(&mut pending.from_the_site, marked);
            }
            area.consenting
        };
        self.note(consenting);
    }
}
