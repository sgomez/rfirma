//! Evaluación y respuesta a los mensajes del canal local (ADR-0005).

use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage, Parameter, SafCode, WireAnswer};

pub use crate::site::domain::channel::ChannelDuty;

/// Respuesta exacta al mensaje de eco del protocolo.
pub const ECHO_OK: &str = "OK";

/// Respuesta del servidor ante un mensaje recibido por el canal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Escribe la respuesta y continúa escuchando.
    Reply(String),
    /// Escribe la respuesta y cierra la conexión.
    ReplyAndClose(String),
    /// La operación se acepta y queda pendiente de resolución.
    Pending(AfirmaUrl),
}

impl Answer {
    /// Texto de la respuesta, si corresponde enviar alguno.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Reply(text) | Self::ReplyAndClose(text) => Some(text),
            Self::Pending(_) => None,
        }
    }
}

/// Determina la respuesta a un mensaje recibido en el canal.
pub fn answer(duty: &ChannelDuty, from_loopback: bool, message: &str) -> Answer {
    if !from_loopback {
        return Answer::ReplyAndClose(
            WireAnswer::refused(SafCode::ExternalRequestToSocket).on_the_wire(),
        );
    }

    let credential = match duty {
        ChannelDuty::Refuse(answer) => return Answer::ReplyAndClose(answer.on_the_wire()),
        ChannelDuty::Serve(credential) => credential,
    };

    let message = ChannelMessage::read(message);
    if message.credential() != Some(credential.as_str()) {
        return Answer::ReplyAndClose(
            WireAnswer::refused_because_of(SafCode::InvalidSessionId, Parameter::IdSession)
                .on_the_wire(),
        );
    }

    match message {
        ChannelMessage::Echo { .. } => Answer::Reply(ECHO_OK.to_owned()),
        ChannelMessage::Operation { url } => Answer::Pending(url),
        ChannelMessage::NotOfTheProtocol => {
            Answer::ReplyAndClose(WireAnswer::refused(SafCode::UnsupportedOperation).on_the_wire())
        }
    }
}

#[cfg(test)]
mod tests;
