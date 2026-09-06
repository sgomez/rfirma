//! Puertos del trámite para el códec del protocolo y el transporte (ADR-0017).

use std::sync::Arc;

use crate::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::protocol::AfirmaUrl;

use super::outcome::SiteOutcome;
use super::request::SiteRequest;

/// Códec del protocolo para decodificar peticiones y codificar desenlaces.
pub trait ProtocolCodec {
    /// Lee la operación que llegó por el canal abierto.
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest;

    /// Línea exacta que se escribe en el canal para el desenlace dado.
    fn encode(&self, outcome: &SiteOutcome) -> String;
}

/// Asa de respuesta única para contestar a la sede y cerrar el canal.
pub struct ReplyHandle(Box<dyn FnOnce(String) + Send>);

impl ReplyHandle {
    /// Crea un asa con la función de entrega dada.
    pub fn of(deliver: impl FnOnce(String) + Send + 'static) -> Self {
        Self(Box::new(deliver))
    }

    /// Contesta a la sede y consume el asa.
    pub fn answer(self, text: String) {
        (self.0)(text);
    }
}

impl std::fmt::Debug for ReplyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReplyHandle")
    }
}

/// Receptor de operaciones entrantes y sus asas de respuesta.
pub type Inbox = Arc<dyn Fn(AfirmaUrl, ReplyHandle) + Send + Sync>;

/// Puerto de transporte para abrir canales de comunicación.
pub trait Transport {
    /// Abre un canal en los puertos indicados para el cometido especificado.
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError>;
}

impl<F> Transport for F
where
    F: Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>,
{
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
        self(ports, duty)
    }
}

/// Referencia al transporte para los casos de uso.
pub type ChannelTransport<'a> =
    &'a dyn Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>;

#[cfg(test)]
mod tests;
