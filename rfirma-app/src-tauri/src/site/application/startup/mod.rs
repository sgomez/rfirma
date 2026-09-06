//! Gestión del flujo de arranque de la aplicación y atención de invocaciones de sede (ADR-0005).

pub mod channel;
pub mod repair;

use std::path::PathBuf;

use crate::site::adapters::tls::LocalCaStore;
use crate::site::domain::trust::Moment as TrustMoment;
use crate::site::ports::TrustStores;

use crate::site::domain::protocol::Refusal;

use super::errand::{Errand, LiveErrand, Moment, NoChannel};
use super::site::{self, Attendance, ChannelTransport};
use super::trust;
use crate::desktop::application::invocation::Invocation;

pub use channel::{hold_the_channel, HeldChannel};
pub use repair::{repair_the_local_ca, LocalCaTrust};

/// Función para abrir y notificar el contenido de la ventana de sede.
pub type SiteWindowOpener<'a> = &'a dyn Fn(SiteWindowContent<'_>);

/// Contenido inicial que debe mostrar la ventana de sede.
#[derive(Debug)]
pub enum SiteWindowContent<'a> {
    /// Trámite activo en servicio.
    TheErrand(&'a Errand),
    /// Trámite bloqueado por una condición irrecuperable.
    ADeadEnd(DeadEnd),
}

/// Situaciones de bloqueo que impiden continuar el trámite con la sede.
#[derive(Debug)]
pub enum DeadEnd {
    /// No se pudo abrir el canal local en los puertos solicitados.
    ChannelNotOpened,
    /// La CA local no está registrada en ningún almacén NSS de confianza (ADR-0005).
    NoLocalCa,
    /// Rechazo de la invocación sin canal disponible para comunicarlo.
    RefusedWithoutChannel(Refusal),
}

/// Estado de disponibilidad de la CA local en los almacenes del sistema (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalCaReach {
    /// La CA local no está presente en ningún almacén revisado.
    Nowhere,
    /// La CA local está presente o no se ha verificado en esta fase.
    NotAnObstacle,
}

/// Parámetros de verificación y almacenes NSS disponibles al arrancar (ADR-0005).
#[derive(Clone, Copy)]
pub struct TrustAtStartup<'a> {
    /// Almacén de la CA local.
    pub store: &'a LocalCaStore,
    /// Rutas de perfiles NSS detectados.
    pub profiles: &'a [PathBuf],
    /// Interfaz de acceso a los almacenes de confianza.
    pub stores: &'a dyn TrustStores,
}

/// Resultado del proceso de arranque de la aplicación.
#[derive(Debug)]
pub struct Startup {
    /// Mensajes informativos sobre el estado de la CA local.
    pub said: Vec<String>,
    /// Ventana seleccionada para abrir en el arranque.
    pub opening: Opening,
}

/// Tipo de ventana que debe abrirse tras evaluar la invocación.
#[derive(Debug)]
pub enum Opening {
    /// Abre la ventana principal para uso local o documento directo.
    TheMainWindow,
    /// Atiende el trámite de sede sin mostrar la ventana principal.
    TheSiteErrand(Attendance),
}

/// Atiende la invocación inicial gestionando la CA local y la ventana correspondiente.
pub fn attend_startup(
    invocation: &Invocation,
    trust: TrustAtStartup<'_>,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
) -> Startup {
    let (said, local_ca) = refresh_the_local_ca(trust);

    let Some(url) = invocation.site_launch() else {
        return Startup {
            said,
            opening: Opening::TheMainWindow,
        };
    };

    Startup {
        said,
        opening: Opening::TheSiteErrand(attend_site_launch(url, transport, window, live, local_ca)),
    }
}

/// Atiende una invocación de sede y abre la ventana asociada según el resultado.
pub fn attend_site_launch(
    url: &str,
    transport: ChannelTransport<'_>,
    window: SiteWindowOpener<'_>,
    live: &LiveErrand,
    local_ca: LocalCaReach,
) -> Attendance {
    let attendance = site::attend_launch(url, transport, live);

    match &attendance {
        Attendance::Serving { errand, .. } => match local_ca {
            LocalCaReach::Nowhere => open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::NoLocalCa),
            ),
            LocalCaReach::NotAnObstacle => open(live, window, SiteWindowContent::TheErrand(errand)),
        },
        Attendance::ChannelNotOpened(_) => {
            open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::ChannelNotOpened),
            );
        }
        Attendance::RefusingInTheWindow(refusal) => {
            open(
                live,
                window,
                SiteWindowContent::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal.clone())),
            );
        }
        Attendance::RefusingOverTheChannel { .. } => {}
    }

    attendance
}

fn open(live: &LiveErrand, window: SiteWindowOpener<'_>, content: SiteWindowContent<'_>) {
    live.note(content.moment());
    window(content);
}

impl SiteWindowContent<'_> {
    /// Devuelve el momento correspondiente para la ventana de sede.
    pub fn moment(&self) -> Moment {
        match self {
            Self::TheErrand(_) => Moment::Waiting,
            Self::ADeadEnd(DeadEnd::ChannelNotOpened) => {
                Moment::NoChannel(NoChannel::ChannelNotOpened)
            }
            Self::ADeadEnd(DeadEnd::NoLocalCa) => Moment::NoChannel(NoChannel::LocalCaMissing),
            Self::ADeadEnd(DeadEnd::RefusedWithoutChannel(refusal)) => {
                Moment::RefusedWithoutChannel(refusal.clone())
            }
        }
    }
}

fn refresh_the_local_ca(trust: TrustAtStartup<'_>) -> (Vec<String>, LocalCaReach) {
    match trust::refresh_local_ca_trust(
        trust.store,
        trust.profiles,
        trust.stores,
        TrustMoment::Startup,
    ) {
        Ok(outcome) => {
            let reach = if outcome.nowhere() {
                LocalCaReach::Nowhere
            } else {
                LocalCaReach::NotAnObstacle
            };
            (
                trust::narrate_startup_outcome(outcome, trust.profiles),
                reach,
            )
        }
        Err(error) => (
            vec![format!(
                "rfirma: no se puede refrescar la CA local ({error}); el arranque sigue sin ella"
            )],
            LocalCaReach::NotAnObstacle,
        ),
    }
}

#[cfg(test)]
mod tests;
