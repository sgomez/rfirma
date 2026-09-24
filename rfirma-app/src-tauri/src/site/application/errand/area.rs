//! El diálogo del área de la firma visible: la que marca la persona, o lo que hace cancelarlo.

use crate::signing::domain::Placement;
use crate::site::domain::protocol::{the_mandatory_area_was_cancelled, IfCancelled};

use super::replies::answering;
use super::{ConsentError, LiveErrand, SiteOutcome};

/// En qué queda el trámite tras el diálogo del área.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterTheArea {
    /// Sigue al consentimiento, que ya está publicado.
    Consenting,
    /// La sede ya tiene su respuesta.
    Answered,
}

/// La persona marca el área de la firma visible, o cancela el diálogo con `None`.
pub fn area_marked(
    live: &LiveErrand,
    marked: Option<&Placement>,
) -> Result<AfterTheArea, ConsentError> {
    let Some(if_cancelled) = live.the_area_to_mark() else {
        return Err(ConsentError::NothingPending);
    };
    if let Some(placement) = marked {
        live.settle_the_area(Some(placement.extra_params()));
        return Ok(AfterTheArea::Consenting);
    }
    match cancel_the_area(live, if_cancelled) {
        Some(refusal) => {
            answering(live, refusal);
            Ok(AfterTheArea::Answered)
        }
        None => Ok(AfterTheArea::Consenting),
    }
}

/// Lo que deja cancelar el diálogo del área que hubiera pendiente.
pub(super) enum PendingAreaCancelled {
    NoneWasPending,
    Consenting,
    Refused(SiteOutcome),
}

/// Cancela el diálogo del área pendiente, si lo hay, como su botón `Cancelar`.
pub(super) fn cancel_the_pending_area(live: &LiveErrand) -> PendingAreaCancelled {
    let Some(if_cancelled) = live.the_area_to_mark() else {
        return PendingAreaCancelled::NoneWasPending;
    };
    cancel_the_area(live, if_cancelled).map_or(
        PendingAreaCancelled::Consenting,
        PendingAreaCancelled::Refused,
    )
}

fn cancel_the_area(live: &LiveErrand, if_cancelled: IfCancelled) -> Option<SiteOutcome> {
    match if_cancelled {
        IfCancelled::Refuses => Some(SiteOutcome::RefusedByTheProtocol(
            the_mandatory_area_was_cancelled(),
        )),
        IfCancelled::SignsWhereTheSiteSaid | IfCancelled::SignsInvisible => {
            live.settle_the_area(None);
            None
        }
    }
}
