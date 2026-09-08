use std::cell::RefCell;

use super::*;
use crate::site::adapters::codec::V4Codec;
use crate::site::adapters::codec_v3::V3Codec;
use crate::site::application::errand::{ProtocolCodec as _, SiteOutcome};
use crate::site::domain::channel::{ChannelLocation, Shutdown, Situation};
use crate::site::domain::protocol::{ChannelCredential, NegotiatedCredential, Parameter, SafCode};
use std::sync::Arc;

fn a_codec_table() -> CodecTable {
    CodecTable {
        v4: Arc::new(V4Codec),
        v3: Arc::new(V3Codec),
        v1: Arc::new(crate::site::adapters::codec_v1::V1Codec),
        relay: Arc::new(|key| {
            Arc::new(crate::site::adapters::codec_relay::RelayCodec::new(key))
                as crate::site::application::errand::NegotiatedCodec
        }),
    }
}

/// Transporte simulado con un cierre para pruebas: ata en el primero de los puertos sorteados,
/// o en el puerto fijo tal cual.
#[derive(Default)]
struct ATransport {
    asked: RefCell<Vec<(ChannelLocation, ChannelDuty)>>,
    refuses: bool,
}

impl ATransport {
    fn that_cannot_bind() -> Self {
        Self {
            refuses: true,
            ..Self::default()
        }
    }

    fn open(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        self.asked.borrow_mut().push((location.clone(), duty));
        if self.refuses {
            return Err(ChannelError::new(
                Situation::NoDrawnPortIsFree,
                "todos ocupados",
            ));
        }
        Ok(OpenChannel::new(
            the_port_bound_at(location),
            Shutdown::of(|| {}),
        ))
    }

    fn asked_once(&self) -> (ChannelLocation, ChannelDuty) {
        let asked = self.asked.borrow();
        assert_eq!(asked.len(), 1, "el transporte se usa una sola vez");
        asked[0].clone()
    }

    fn was_never_asked(&self) {
        assert!(self.asked.borrow().is_empty(), "no habia donde abrir nada");
    }
}

fn the_port_bound_at(location: &ChannelLocation) -> u16 {
    match location {
        ChannelLocation::Fixed(port) => *port,
        ChannelLocation::Drawn(ports) | ChannelLocation::Service(ports) => {
            *ports.first().expect("se ata uno de los sorteados")
        }
        ChannelLocation::Relay(_) => 0,
    }
}

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

fn a_launch(parameters: &str) -> String {
    format!("afirma://websocket?{parameters}")
}

/// La tabla de negociación es la prueba central del ticket: cada forma de invocación de arranque
/// decide un (códec, ubicación de canal) o un rechazo, en un solo sitio.
#[test]
fn the_negotiation_table_decides_codec_and_location_by_the_shape_of_the_launch() {
    struct Case {
        name: &'static str,
        url: String,
        expected: Expected,
    }

    enum Expected {
        Codec {
            location: ChannelLocation,
            codec_is_v3: bool,
        },
        RefusedOverTheChannel {
            location: ChannelLocation,
            code: SafCode,
        },
        RefusedInTheWindow(SafCode),
    }

    let cases = vec![
        Case {
            name: "v4 con puertos e idsession negocia el codec de la cuarta version",
            url: a_launch(&format!("ports=54001,54002&v=4&idsession={CREDENTIAL}")),
            expected: Expected::Codec {
                location: ChannelLocation::Drawn(vec![54001, 54002]),
                codec_is_v3: false,
            },
        },
        Case {
            name: "v3 sin puertos negocia el codec de la tercera version en el puerto fijo",
            url: a_launch(&format!("v=3&idsession={CREDENTIAL}")),
            expected: Expected::Codec {
                location: ChannelLocation::Fixed(
                    crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL,
                ),
                codec_is_v3: true,
            },
        },
        Case {
            name: "v3 sin idsession tambien negocia, sin credencial",
            url: a_launch("v=3"),
            expected: Expected::Codec {
                location: ChannelLocation::Fixed(
                    crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL,
                ),
                codec_is_v3: true,
            },
        },
        Case {
            name: "verbo desconocido se rechaza en la ventana",
            url: "afirma://sign?v=4&idsession=abc".to_owned(),
            expected: Expected::RefusedInTheWindow(SafCode::Params),
        },
        Case {
            name: "una version no soportada con puertos se rechaza por el canal",
            url: a_launch("ports=54001&v=99&idsession=abc"),
            expected: Expected::RefusedOverTheChannel {
                location: ChannelLocation::Drawn(vec![54001]),
                code: SafCode::UnsupportedProcedure,
            },
        },
        Case {
            name: "v4 sin ports se rechaza en la ventana: no hay puerto candidato",
            url: a_launch(&format!("v=4&idsession={CREDENTIAL}")),
            expected: Expected::RefusedInTheWindow(SafCode::Params),
        },
        Case {
            name: "v4 sin idsession se rechaza por el canal que si trae puertos",
            url: a_launch("ports=54001&v=4"),
            expected: Expected::RefusedOverTheChannel {
                location: ChannelLocation::Drawn(vec![54001]),
                code: SafCode::Params,
            },
        },
        Case {
            name: "un idsession mal formado en v3 se rechaza por el puerto fijo",
            url: a_launch("v=3&idsession=mal-formado"),
            expected: Expected::RefusedOverTheChannel {
                location: ChannelLocation::Fixed(
                    crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL,
                ),
                code: SafCode::Params,
            },
        },
    ];

    for case in cases {
        let transport = ATransport::default();
        let attendance = attend_launch(
            &case.url,
            &a_codec_table(),
            &|location, duty| transport.open(location, duty),
            &LiveErrand::default(),
        );

        match case.expected {
            Expected::Codec {
                location,
                codec_is_v3,
            } => {
                let Attendance::Serving { errand, .. } = &attendance else {
                    panic!("{}: se esperaba servir, salio {attendance:?}", case.name);
                };
                let (asked_location, _) = transport.asked_once();
                assert_eq!(asked_location, location, "{}", case.name);
                assert_eq!(
                    errand.codec().encode(&SiteOutcome::Cancelled),
                    if codec_is_v3 {
                        V3Codec.encode(&SiteOutcome::Cancelled)
                    } else {
                        V4Codec.encode(&SiteOutcome::Cancelled)
                    },
                    "{}",
                    case.name
                );
            }
            Expected::RefusedOverTheChannel { location, code } => {
                let Attendance::RefusingOverTheChannel { answer, .. } = &attendance else {
                    panic!(
                        "{}: se esperaba canal de rechazo, salio {attendance:?}",
                        case.name
                    );
                };
                assert!(
                    answer
                        .on_the_wire()
                        .starts_with(&WireAnswer::refused(code).on_the_wire()),
                    "{}: {answer:?}",
                    case.name
                );
                let (asked_location, _) = transport.asked_once();
                assert_eq!(asked_location, location, "{}", case.name);
            }
            Expected::RefusedInTheWindow(code) => {
                let Attendance::RefusingInTheWindow(refusal) = &attendance else {
                    panic!(
                        "{}: se esperaba la ventana, salio {attendance:?}",
                        case.name
                    );
                };
                assert_eq!(refusal.code(), code, "{}", case.name);
                transport.was_never_asked();
            }
        }
    }
}

#[test]
fn the_negotiation_table_picks_the_relay_codec_for_an_operation_with_servlet() {
    let transport = ATransport::default();
    let url = "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.\
               example/store&id=tx-1&key=12345678";

    let attendance = attend_launch(
        url,
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::Serving { errand, .. } = &attendance else {
        panic!("se esperaba servir, salio {attendance:?}");
    };
    let (asked_location, _) = transport.asked_once();
    assert!(matches!(asked_location, ChannelLocation::Relay(_)));

    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
    };
    let key = crate::site::domain::protocol::CipherKey::from_url_parameter("12345678")
        .expect("longitud correcta")
        .expect("un valor no vacio siempre produce clave");
    assert_eq!(
        errand.codec().encode(&outcome),
        crate::site::adapters::codec_relay::RelayCodec::new(Some(key)).encode(&outcome),
        "la fila de servidor intermedio cifra con la clave que trajo la url"
    );
}

#[test]
fn a_relay_launch_delivers_only_after_the_errand_is_registered() {
    use crate::site::adapters::relay::Relay;
    use crate::site::application::tests::InMemoryServlets;
    use crate::site::ports::{Inbox, ReplyHandle, Transport as _};
    use std::sync::Mutex;

    let live = Arc::new(LiveErrand::default());
    let codec_was_already_registered = Arc::new(Mutex::new(None));

    let inbox: Inbox = {
        let live = Arc::clone(&live);
        let seen = Arc::clone(&codec_was_already_registered);
        Arc::new(move |_url, reply: ReplyHandle| {
            *seen.lock().expect("el candado") = Some(live.codec().is_some());
            reply.answer("respuesta".to_owned());
        })
    };
    let relay = Relay::new(
        Arc::new(InMemoryServlets::default()),
        inbox,
        Arc::new(|| {}),
        Arc::new(|_refusal| {}),
    );

    let url = "afirma://sign?algorithm=SHA256withRSA&dat=ZmlybWFkbw&stservlet=https://relay.\
               example/store&id=tx-order&key=12345678";

    let attendance = attend_launch(
        url,
        &a_codec_table(),
        &|location, duty| relay.open(location, duty),
        &live,
    );

    assert!(matches!(attendance, Attendance::Serving { .. }));
    assert_eq!(
        *codec_was_already_registered.lock().expect("el candado"),
        Some(true),
        "la entrega debe llegar despues de registrar el tramite, no antes"
    );
}

#[test]
fn a_good_launch_opens_the_channel_on_one_of_the_drawn_ports() {
    let transport = ATransport::default();

    let attendance = attend_launch(
        &a_launch(&format!(
            "ports=54001,54002,54003&v=4&idsession={CREDENTIAL}"
        )),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::Serving { channel, .. } = attendance else {
        panic!("la invocacion era buena: {attendance:?}");
    };
    assert_eq!(channel.port(), 54001);
    assert_eq!(
        transport.asked_once(),
        (
            ChannelLocation::Drawn(vec![54001, 54002, 54003]),
            ChannelDuty::Serve(NegotiatedCredential::Required(
                ChannelCredential::parse(CREDENTIAL).expect("la credencial es buena")
            ))
        ),
        "el canal se cierra con la credencial que trajo la URL"
    );
}

#[test]
fn a_good_third_protocol_launch_opens_the_channel_on_the_fixed_port_without_credential() {
    let transport = ATransport::default();

    let attendance = attend_launch(
        &a_launch("v=3"),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::Serving { channel, .. } = attendance else {
        panic!("la invocacion era buena: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL
    );
    assert_eq!(
        transport.asked_once(),
        (
            ChannelLocation::Fixed(crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL),
            ChannelDuty::Serve(NegotiatedCredential::Absent)
        ),
        "sin idsession el canal no exige credencial"
    );
}

#[test]
fn without_drawn_ports_the_refusal_is_only_shown_in_the_window() {
    let transport = ATransport::default();

    let attendance = attend_launch(
        &a_launch(&format!("v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
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
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
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
fn a_malformed_credential_in_the_third_protocol_is_refused_over_the_fixed_channel() {
    let transport = ATransport::default();

    let attendance = attend_launch(
        &a_launch("v=3&idsession=no-vale-esta"),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::RefusingOverTheChannel { channel, answer } = attendance else {
        panic!("el protocolo 3 siempre tiene un canal fijo: {attendance:?}");
    };
    assert_eq!(
        channel.port(),
        crate::site::domain::protocol::THE_PORT_OF_THE_THIRD_PROTOCOL
    );
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
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
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
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
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
        &a_launch("ports=54001&v=99&idsession=abc"),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let Attendance::RefusingInTheWindow(refusal) = attendance else {
        panic!("sin canal disponible no hay socket: {attendance:?}");
    };
    assert_eq!(refusal.code(), SafCode::UnsupportedProcedure);
}

#[test]
fn the_ports_that_reach_the_transport_are_the_ones_the_url_carried() {
    let transport = ATransport::default();

    let _ = attend_launch(
        &a_launch(&format!("ports=54001,54002&v=4&idsession={CREDENTIAL}")),
        &a_codec_table(),
        &|location, duty| transport.open(location, duty),
        &LiveErrand::default(),
    );

    let (location, _) = transport.asked_once();
    let ChannelLocation::Drawn(ports) = location else {
        panic!("esta prueba sortea puertos: {location:?}");
    };
    assert_eq!(ports, vec![54001, 54002]);
    assert!(
        !ports.contains(&crate::site::adapters::channel::THE_PORT_OF_THE_THIRD_PROTOCOL),
        "el puerto fijo del protocolo 3 no sale de ninguna parte"
    );
}
