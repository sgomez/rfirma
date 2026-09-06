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
mod tests {
    use super::*;

    #[test]
    fn what_is_answered_is_what_the_other_end_receives() {
        let received = std::sync::Arc::new(std::sync::Mutex::new(None));
        let keeping = std::sync::Arc::clone(&received);
        let handle = ReplyHandle::of(move |text| {
            *keeping.lock().expect("el candado") = Some(text);
        });

        handle.answer("OK".to_owned());

        assert_eq!(received.lock().expect("el candado").as_deref(), Some("OK"));
    }

    #[test]
    fn a_closure_with_the_right_shape_is_a_transport() {
        let transport = |ports: &[u16], _duty: ChannelDuty| {
            Ok(OpenChannel::new(
                ports[0],
                crate::channel::Shutdown::of(|| {}),
            ))
        };
        let opened = Transport::open(
            &transport,
            &[51001],
            ChannelDuty::Refuse(crate::protocol::WireAnswer::refused(
                crate::protocol::SafCode::CannotOpenSocket,
            )),
        )
        .expect("abre");
        assert_eq!(opened.port(), 51001);
    }
}
