//! Reparación e instalación manual de la CA local desde la interfaz de sede (ADR-0005).

use std::path::PathBuf;

use crate::site::domain::trust::Moment as TrustMoment;
use crate::site::ports::{LocalCaSlots, TrustStores};

use super::super::errand::{LiveErrand, Moment, NoChannel};
use super::super::trust;
use super::channel::HeldChannel;

/// La CA local, sus almacenes y los perfiles NSS que la reparación toca (ADR-0005).
pub struct LocalCaTrust {
    /// Las dos ranuras de la CA local.
    pub store: Box<dyn LocalCaSlots + Send + Sync>,
    /// Perfiles NSS de navegadores detectados.
    pub profiles: Vec<PathBuf>,
    /// Los almacenes de confianza donde se registra.
    pub stores: Box<dyn TrustStores + Send + Sync>,
}

/// Instala la CA local a petición de la persona y actualiza el estado de la ventana (ADR-0005).
pub fn repair_the_local_ca(trust: &LocalCaTrust, held: &HeldChannel, live: &LiveErrand) -> Moment {
    let in_some_store = trust::refresh_local_ca_trust(
        trust.store.as_ref(),
        &trust.profiles,
        trust.stores.as_ref(),
        TrustMoment::Startup,
    )
    .is_ok_and(|outcome| !outcome.nowhere());

    let moment = what_the_repair_leaves(in_some_store, held.is_serving());
    live.note(moment.clone());
    moment
}

fn what_the_repair_leaves(in_some_store: bool, channel_is_serving: bool) -> Moment {
    match (in_some_store, channel_is_serving) {
        (false, _) => Moment::NoChannel(NoChannel::LocalCaMissing),
        (true, true) => Moment::Waiting,
        (true, false) => Moment::NoChannel(NoChannel::ChannelNotOpened),
    }
}

#[cfg(test)]
mod tests;
