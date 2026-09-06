//! Invocación de sede por esquema de URL y negociación de canal (ADR-0005, ADR-0017).

use std::sync::Arc;

use crate::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::protocol::{
    drawn_ports, AfirmaUrl, ChannelCredential, LaunchRequest, Refusal, RefusalSituation, SafCode,
    WireAnswer,
};

use super::errand::{Errand, LiveErrand, NegotiatedCodec};
use crate::app::codec::V4Codec;

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
pub fn negotiate(url: &AfirmaUrl) -> Result<Negotiated, Refusal> {
    let request = LaunchRequest::from_url(url)?;
    Ok(Negotiated {
        codec: Arc::new(V4Codec),
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
pub fn attend_launch(url: &str, transport: ChannelTransport<'_>, live: &LiveErrand) -> Attendance {
    let url = match AfirmaUrl::parse(url) {
        Ok(url) => url,
        Err(refusal) => return Attendance::RefusingInTheWindow(refusal),
    };

    match negotiate(&url) {
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
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::channel::{Shutdown, Situation};
    use crate::protocol::{ChannelCredential, Parameter, SafCode};

    /// Transporte simulado con un cierre para pruebas.
    #[derive(Default)]
    struct ATransport {
        asked: RefCell<Vec<(Vec<u16>, ChannelDuty)>>,
        refuses: bool,
    }

    impl ATransport {
        fn that_cannot_bind() -> Self {
            Self {
                refuses: true,
                ..Self::default()
            }
        }

        fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
            self.asked.borrow_mut().push((ports.to_vec(), duty));
            if self.refuses {
                return Err(ChannelError::new(
                    Situation::NoDrawnPortIsFree,
                    "todos ocupados",
                ));
            }
            Ok(OpenChannel::new(
                *ports.first().expect("se ata uno de los sorteados"),
                Shutdown::of(|| {}),
            ))
        }

        fn asked_once(&self) -> (Vec<u16>, ChannelDuty) {
            let asked = self.asked.borrow();
            assert_eq!(asked.len(), 1, "el transporte se usa una sola vez");
            asked[0].clone()
        }

        fn was_never_asked(&self) {
            assert!(
                self.asked.borrow().is_empty(),
                "no habia puertos: no se podia abrir nada"
            );
        }
    }

    const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

    fn a_launch(parameters: &str) -> String {
        format!("afirma://websocket?{parameters}")
    }

    #[test]
    fn a_good_launch_opens_the_channel_on_one_of_the_drawn_ports() {
        let transport = ATransport::default();

        let attendance = attend_launch(
            &a_launch(&format!(
                "ports=54001,54002,54003&v=4&idsession={CREDENTIAL}"
            )),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::Serving { channel, .. } = attendance else {
            panic!("la invocacion era buena: {attendance:?}");
        };
        assert_eq!(channel.port(), 54001);
        assert_eq!(
            transport.asked_once(),
            (
                vec![54001, 54002, 54003],
                ChannelDuty::Serve(
                    ChannelCredential::parse(CREDENTIAL).expect("la credencial es buena")
                )
            ),
            "el canal se cierra con la credencial que trajo la URL"
        );
    }

    #[test]
    fn a_refusal_is_answered_over_the_socket_when_the_site_drew_ports() {
        let transport = ATransport::default();

        let attendance = attend_launch(
            &a_launch(&format!("ports=54001,54002&v=3&idsession={CREDENTIAL}")),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::RefusingOverTheChannel { channel, answer } = attendance else {
            panic!("hay puertos, asi que hay socket: {attendance:?}");
        };
        assert_eq!(answer, WireAnswer::refused(SafCode::UnsupportedProcedure));
        assert_eq!(channel.port(), 54001);
        assert_eq!(
            transport.asked_once(),
            (
                vec![54001, 54002],
                ChannelDuty::Refuse(WireAnswer::refused(SafCode::UnsupportedProcedure))
            ),
            "ese canal no sirve la conversacion: sólo contesta el codigo"
        );
    }

    #[test]
    fn without_drawn_ports_the_refusal_is_only_shown_in_the_window() {
        let transport = ATransport::default();

        let attendance = attend_launch(
            &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::RefusingInTheWindow(refusal) = attendance else {
            panic!("sin puertos no hay socket: {attendance:?}");
        };
        assert_eq!(refusal.code(), SafCode::Params);
        transport.was_never_asked();
    }

    #[test]
    fn a_malformed_credential_is_refused_over_the_socket() {
        let transport = ATransport::default();

        let attendance = attend_launch(
            &a_launch("ports=54001&v=4&idsession=no-vale-esta"),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::RefusingOverTheChannel { answer, .. } = attendance else {
            panic!("habia puertos: {attendance:?}");
        };
        assert_eq!(
            answer,
            WireAnswer::refused_because_of(SafCode::Params, Parameter::IdSession)
        );
    }

    #[test]
    fn something_that_is_not_a_protocol_url_never_reaches_the_transport() {
        let transport = ATransport::default();

        let attendance = attend_launch(
            "https://sede.example/firmar",
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        assert!(matches!(attendance, Attendance::RefusingInTheWindow(_)));
        transport.was_never_asked();
    }

    #[test]
    fn a_good_launch_with_every_port_taken_has_no_channel_to_speak_through() {
        let transport = ATransport::that_cannot_bind();

        let attendance = attend_launch(
            &a_launch(&format!("ports=54001&v=4&idsession={CREDENTIAL}")),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::ChannelNotOpened(error) = attendance else {
            panic!("no se ha podido atar nada: {attendance:?}");
        };
        assert_eq!(error.situation(), Situation::NoDrawnPortIsFree);
    }

    #[test]
    fn a_refusal_that_cannot_be_answered_over_a_socket_falls_back_to_the_window() {
        let transport = ATransport::that_cannot_bind();

        let attendance = attend_launch(
            &a_launch(&format!("ports=54001&v=3&idsession={CREDENTIAL}")),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let Attendance::RefusingInTheWindow(refusal) = attendance else {
            panic!("sin puerto no hay socket: {attendance:?}");
        };
        assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
    }

    #[test]
    fn the_ports_that_reach_the_transport_are_the_ones_the_url_carried() {
        let transport = ATransport::default();

        let _ = attend_launch(
            &a_launch(&format!("ports=54001,54002&v=3&idsession={CREDENTIAL}")),
            &|ports, duty| transport.open(ports, duty),
            &LiveErrand::default(),
        );

        let (ports, _) = transport.asked_once();
        assert_eq!(ports, vec![54001, 54002]);
        assert!(
            !ports.contains(&crate::channel::THE_PORT_OF_THE_THIRD_PROTOCOL),
            "el puerto fijo del protocolo 3 no sale de ninguna parte"
        );
    }
}
