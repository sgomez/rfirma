//! Invocación de sede por esquema de URL y negociación de canal y códec (ADR-0005, ADR-0017).

use crate::site::domain::channel::{ChannelDuty, ChannelError, ChannelLocation, OpenChannel};
use crate::site::domain::protocol::{
    location_for_a_refusal, AfirmaUrl, LaunchRequest, NegotiatedCredential, Refusal,
    RefusalSituation, SafCode, WireAnswer,
};

use super::errand::{Errand, LiveErrand, NegotiatedCodec};

pub use super::errand::ChannelTransport;

/// La tabla de adaptadores que la raíz de composición entrega a la negociación: el códec que
/// habla cada forma de invocación de arranque. Un verbo o un transporte nuevos son una fila más.
#[derive(Clone)]
pub struct CodecTable {
    /// Códec de la versión 4: puertos sorteados por la sede.
    pub v4: NegotiatedCodec,
    /// Códec de la versión 3: puerto fijo, sin sorteo.
    pub v3: NegotiatedCodec,
}

impl CodecTable {
    fn codec_for(&self, location: &ChannelLocation) -> NegotiatedCodec {
        match location {
            ChannelLocation::Drawn(_) => self.v4.clone(),
            ChannelLocation::Fixed(_) => self.v3.clone(),
        }
    }
}

/// Resultado de la negociación de protocolo y canal para una invocación.
pub struct Negotiated {
    /// Códec acordado para leer operaciones y escribir respuestas.
    pub codec: NegotiatedCodec,
    /// Dónde escuchará el canal.
    pub location: ChannelLocation,
    /// Credencial negociada para autenticar la sesión, si la sede la exige.
    pub credential: NegotiatedCredential,
}

/// Negocia el códec y los parámetros de canal a partir de la forma de la URL de invocación.
pub fn negotiate(url: &AfirmaUrl, codecs: &CodecTable) -> Result<Negotiated, Refusal> {
    let request = LaunchRequest::from_url(url)?;
    Ok(Negotiated {
        codec: codecs.codec_for(request.location()),
        location: request.location().clone(),
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
    codecs: &CodecTable,
    transport: ChannelTransport<'_>,
    live: &LiveErrand,
) -> Attendance {
    let url = match AfirmaUrl::parse(url) {
        Ok(url) => url,
        Err(refusal) => return Attendance::RefusingInTheWindow(refusal),
    };

    match negotiate(&url, codecs) {
        Ok(negotiated) => {
            let duty = ChannelDuty::Serve(negotiated.credential.clone());
            match transport(&negotiated.location, duty) {
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
    let Some(location) = location_for_a_refusal(url) else {
        return Attendance::RefusingInTheWindow(refusal);
    };

    match transport(&location, ChannelDuty::Refuse(refusal.answer())) {
        Ok(channel) => Attendance::RefusingOverTheChannel {
            channel,
            answer: refusal.answer(),
        },
        Err(_) => Attendance::RefusingInTheWindow(refusal),
    }
}

#[cfg(test)]
mod tests;
