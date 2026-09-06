//! **El transporte de producción** (RD-04, ID-213, ID-326): `wss://` sobre el
//! *loopback*, en uno de los puertos que sorteó la sede.
//!
//! Es el adaptador de [`Transport`] que envuelve a [`crate::channel`] —el
//! servidor, la conversación y sus tres guardias no cambian—: aquí sólo se
//! fabrica el certificado del servidor local, se abre el canal y se traduce el
//! asa del servidor al [`ReplyHandle`] del puerto. Lo que llega por el canal se
//! entrega por el [`Inbox`] con el que se construyó, que es como el trámite
//! recibe la operación sin nombrar a este módulo.
//!
//! El certificado del servidor local se fabrica **al abrir** y no antes de
//! arrancar: la CA que lo firma puede estar naciendo en este mismo arranque
//! —el refresco es lo primero que hace el caso de uso de arranque—, así que
//! leerla antes sería leerla antes de que exista.

use std::sync::Arc;

use crate::channel::{self, ChannelDuty, ChannelError, OpenChannel};
use crate::tls::{LocalCaStore, LocalServerCertificate};

use super::errand::{Inbox, ReplyHandle, Transport};

/// El `wss` sobre el *loopback* con puerto sorteado.
pub struct LoopbackWss {
    store: LocalCaStore,
    inbox: Inbox,
}

impl LoopbackWss {
    /// El transporte que firma su certificado con esa CA local y entrega lo que
    /// llegue por ese buzón.
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

        // El asa del servidor se convierte en la del puerto: el trámite escribe
        // por la segunda sin saber que detrás hay un `oneshot` (RD-04).
        let inbox = Arc::clone(&self.inbox);
        let operations: channel::SiteOperations = Arc::new(move |url, reply| {
            inbox(url, ReplyHandle::of(move |text| reply.answer(text)));
        });

        channel::open(ports, &certificate, duty, operations)
    }
}
