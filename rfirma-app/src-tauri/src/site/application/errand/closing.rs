//! El cierre de la ventana de sede por el gestor de ventanas con el trámite vivo.

use std::time::Duration;

use super::area::{cancel_the_pending_area, PendingAreaCancelled};
use super::{dismiss_the_warning, LiveErrand, SiteOutcome};

/// Tope de espera al acuse de entrega antes de cerrar la ventana de sede por el gestor de ventanas.
pub const WINDOW_CLOSE_ACKNOWLEDGEMENT_TIMEOUT: Duration = Duration::from_secs(1);

/// Qué le queda a la ventana de sede tras contestar su cierre por el gestor de ventanas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowAfterClosing {
    /// Se cierra, y con ella el proceso.
    Closes,
    /// Ya se ha ocultado, porque el canal sigue sirviendo (ADR-0024).
    StaysHidden,
    /// Sigue abierta, porque el cierre solo cancelaba el diálogo del área.
    StaysOpen,
}

/// Contesta a la sede antes de cerrar la ventana: el rechazo que enseñaba, `CANCEL` si había algo que consentir, o lo que haga cancelar el diálogo del área.
pub fn answer_before_closing(live: &LiveErrand) -> WindowAfterClosing {
    answer_before_closing_within(live, WINDOW_CLOSE_ACKNOWLEDGEMENT_TIMEOUT)
}

pub(super) fn answer_before_closing_within(
    live: &LiveErrand,
    timeout: Duration,
) -> WindowAfterClosing {
    if dismiss_the_warning(live) {
        return WindowAfterClosing::StaysHidden;
    }
    if live.current().is_none() {
        return WindowAfterClosing::Closes;
    }
    let outcome = match cancel_the_pending_area(live) {
        PendingAreaCancelled::Consenting => return WindowAfterClosing::StaysOpen,
        PendingAreaCancelled::Refused(refusal) => refusal,
        PendingAreaCancelled::NoneWasPending => live
            .the_shown_refusal()
            .map_or(SiteOutcome::Cancelled, SiteOutcome::RefusedByTheProtocol),
    };
    if live.keeps_serving() {
        live.answer_once_put_away(&outcome);
        return WindowAfterClosing::StaysHidden;
    }
    live.answer_the_site(&outcome);
    live.wait_for_delivery(timeout);
    live.end();
    WindowAfterClosing::Closes
}
