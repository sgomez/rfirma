//! Transporte de producción para el servidor local HTTPS y WebSockets (ADR-0005, ADR-0017).

use crate::site::adapters::channel;
use crate::site::adapters::tls::{LocalCaStore, LocalServerCertificate};
use crate::site::domain::channel::{ChannelDuty, ChannelError, ChannelLocation, OpenChannel};

use crate::site::application::errand::{Inbox, Transport};

/// Transporte WSS sobre la interfaz local con puerto sorteado.
pub struct LoopbackWss {
    store: LocalCaStore,
    inbox: Inbox,
}

impl LoopbackWss {
    /// Crea un transporte que emite su certificado con la CA local dada.
    pub fn new(store: LocalCaStore, inbox: Inbox) -> Self {
        Self { store, inbox }
    }
}

impl Transport for LoopbackWss {
    fn open(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        let unusable = |detail: String| {
            ChannelError::new(
                crate::site::domain::channel::Situation::MaterialNotUsable,
                detail,
            )
        };
        let ca = self
            .store
            .read()
            .map_err(|error| unusable(error.to_string()))?
            .ok_or_else(|| {
                unusable(
                    "no hay CA local con la que firmar el certificado del servidor local"
                        .to_owned(),
                )
            })?;
        let certificate =
            LocalServerCertificate::issued_by(&ca).map_err(|error| unusable(error.to_string()))?;

        channel::open(location, &certificate, duty, self.inbox.clone())
    }
}
