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
    match (marked, if_cancelled) {
        (Some(placement), _) => live.settle_the_area(Some(placement.extra_params())),
        (None, IfCancelled::Refuses) => {
            answering(
                live,
                SiteOutcome::RefusedByTheProtocol(the_mandatory_area_was_cancelled()),
            );
            return Ok(AfterTheArea::Answered);
        }
        (None, IfCancelled::SignsWhereTheSiteSaid | IfCancelled::SignsInvisible) => {
            live.settle_the_area(None);
        }
    }
    Ok(AfterTheArea::Consenting)
}
