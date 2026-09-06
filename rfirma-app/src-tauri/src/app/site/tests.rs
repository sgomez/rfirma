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
