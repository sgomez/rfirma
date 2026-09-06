//! Transporte de producción para el servidor local HTTPS y WebSockets (ADR-0005, ADR-0017).

use std::sync::Arc;

use crate::site::adapters::channel;
use crate::site::adapters::tls::{LocalCaStore, LocalServerCertificate};
use crate::site::domain::channel::{ChannelDuty, ChannelError, OpenChannel};

use crate::site::application::errand::{Inbox, ReplyHandle, Transport};

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
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
        let unusable =
            |detail: String| ChannelError::new(channel::Situation::MaterialNotUsable, detail);
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

        let inbox = Arc::clone(&self.inbox);
        let operations: channel::SiteOperations = Arc::new(move |url, reply| {
            inbox(url, ReplyHandle::of(move |text| reply.answer(text)));
        });

        channel::open(ports, &certificate, duty, operations)
    }
}
