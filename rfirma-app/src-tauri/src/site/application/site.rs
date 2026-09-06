//! Invocación de sede por esquema de URL y negociación de canal (ADR-0005, ADR-0017).

use crate::site::domain::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::site::domain::protocol::{
    drawn_ports, AfirmaUrl, ChannelCredential, LaunchRequest, Refusal, RefusalSituation, SafCode,
    WireAnswer,
};

use super::errand::{Errand, LiveErrand, NegotiatedCodec};

pub use super::errand::ChannelTransport;

/// Resultado de la negociación de protocolo y canal para una invocación.
pub struct Negotiated {
    /// Códec acordado para leer operaciones y escribir respuestas.
    pub codec: NegotiatedCodec,
    /// Puertos sorteados por la sede.
    pub ports: Vec<u16>,
    /// Credencial para autenticar la sesión.
    pub credential: ChannelCredential,
}

/// Negocia el protocolo y parámetros de canal a partir de la URL de invocación.
pub fn negotiate(url: &AfirmaUrl, codec: &NegotiatedCodec) -> Result<Negotiated, Refusal> {
    let request = LaunchRequest::from_url(url)?;
    Ok(Negotiated {
        codec: codec.clone(),
        ports: request.ports().to_vec(),
        credential: request.credential().clone(),
    })
}

/// Desenlace del intento de atención a la invocación de la sede.
#[derive(Debug)]
pub enum Attendance {
    /// Canal abierto sirviendo la conversación con la sede.
    Serving {
        /// Canal abierto para la sesión.
        channel: OpenChannel,
        /// Trámite activo registrado.
        errand: Errand,
    },
    /// Canal abierto exclusivamente para comunicar un rechazo.
    RefusingOverTheChannel {
        /// Canal abierto para responder.
        channel: OpenChannel,
        /// Respuesta de rechazo a enviar.
        answer: WireAnswer,
    },
    /// Rechazo notificado a través de la ventana por falta de canal.
    RefusingInTheWindow(Refusal),
    /// Error al intentar abrir el canal de comunicación.
    ChannelNotOpened(ChannelError),
}

/// Atiende la invocación de arranque recibida por el protocolo afirma://.
pub fn attend_launch(
    url: &str,
    codec: &NegotiatedCodec,
    transport: ChannelTransport<'_>,
    live: &LiveErrand,
) -> Attendance {
    let url = match AfirmaUrl::parse(url) {
        Ok(url) => url,
        Err(refusal) => return Attendance::RefusingInTheWindow(refusal),
    };

    match negotiate(&url, codec) {
        Ok(negotiated) => {
            let duty = ChannelDuty::Serve(negotiated.credential.clone());
            match transport(&negotiated.ports, duty) {
                Ok(channel) => {
                    let errand =
                        Errand::of(negotiated.credential, channel.port(), negotiated.codec);
                    if live.begin(errand.clone()) {
                        return Attendance::Serving { channel, errand };
                    }

                    channel.close();
                    refuse(
                        &url,
                        Refusal::new(
                            SafCode::CannotOpenSocket,
                            "ya hay un tramite de sede vivo: no se atienden dos a la vez",
                        )
                        .because(RefusalSituation::ErrandInFlight),
                        transport,
                    )
                }
                Err(error) => Attendance::ChannelNotOpened(error),
            }
        }
        Err(refusal) => refuse(&url, refusal, transport),
    }
}

fn refuse(url: &AfirmaUrl, refusal: Refusal, transport: ChannelTransport<'_>) -> Attendance {
    let ports = drawn_ports(url);
    if ports.is_empty() {
        return Attendance::RefusingInTheWindow(refusal);
    }

    match transport(&ports, ChannelDuty::Refuse(refusal.answer())) {
        Ok(channel) => Attendance::RefusingOverTheChannel {
            channel,
            answer: refusal.answer(),
        },
        Err(_) => Attendance::RefusingInTheWindow(refusal),
    }
}

#[cfg(test)]
mod tests;
