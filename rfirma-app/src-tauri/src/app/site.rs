//! **La invocación de una sede**: qué se hace con un `afirma://websocket?…`
//! (ID-214, ID-215, ID-248).
//!
//! Este caso de uso decide tres cosas y ninguna más:
//!
//! 1. Si la invocación se acepta, el canal se abre **en uno de los puertos que
//!    la sede sorteó** y sirve la conversación, cerrada con la credencial de
//!    canal que trajo la URL.
//! 2. Si se rechaza, **el rechazo se contesta por el socket cuando hay socket**:
//!    con `ports` en la URL, se abre el canal igual y el primer mensaje se lleva
//!    el `SAF_`. Es más compatible que el original, que se mata y deja a la sede
//!    esperando treinta segundos un `ApplicationNotFoundException` falso.
//! 3. **Sólo por ventana cuando la URL no trae `ports`** —el caso `v=3`, porque
//!    AutoFirma asume la 3 justamente *cuando `ports` falta*—: sin puertos no
//!    hay por dónde contestar.
//!
//! # El servidor es un puerto
//!
//! El caso de uso no construye el servidor: lo recibe como
//! [`ChannelTransport`], un cierre, igual que [`crate::app::version`] recibe la
//! red (ID-182). Es **la única costura nueva del hito** (ID-214, TD-51), y es
//! lo que hace que ninguna prueba de aquí abra un socket (TD-52): las de este
//! módulo doblan el transporte con un cierre que apunta lo que se le pidió.
//! En producción lo cumple [`crate::channel`].

use crate::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::protocol::{
    drawn_ports, AfirmaUrl, LaunchRequest, Refusal, RefusalSituation, SafCode, WireAnswer,
};

use super::errand::{Errand, LiveErrand};

/// **El puerto de transporte** (ID-214): ata uno de esos puertos y sirve el
/// canal para ese cometido.
pub type ChannelTransport<'a> =
    &'a dyn Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>;

/// En qué queda la invocación de una sede.
#[derive(Debug)]
pub enum Attendance {
    /// El canal está abierto y sirviendo la conversación.
    Serving {
        /// El canal abierto, que sirve la conversación de la sede.
        channel: OpenChannel,
        /// El trámite que se quedó con la plaza (ID-280): el que apuntó
        /// [`LiveErrand::begin`], no el que se intentó.
        errand: Errand,
    },
    /// La invocación se rechaza, y el rechazo va **por el socket**: el canal
    /// está abierto sólo para decir el código al primer mensaje y cerrar.
    RefusingOverTheChannel {
        /// El canal abierto para contestar, que no expone ninguna capacidad.
        channel: OpenChannel,
        /// Lo que se contestará: el código del catálogo y, si el rechazo es
        /// de un parámetro, cuál (ID-290).
        answer: WireAnswer,
    },
    /// La invocación se rechaza y **no hay socket** por el que decirlo: sin
    /// `ports` en la URL, o con todos ocupados. Lo cuenta la ventana.
    RefusingInTheWindow(Refusal),
    /// La invocación era buena pero el canal no se ha podido abrir. También es
    /// cosa de la ventana: no hay canal por el que hablar.
    ChannelNotOpened(ChannelError),
}

/// Atiende la invocación de arranque que llegó por el esquema `afirma://`.
///
/// `live` es el trámite de sede a medias, si lo hay: **con uno vivo, la
/// invocación se rechaza** con `SAF_45` mientras el primero siga vivo (ID-280).
/// Para la sede es exactamente eso, que no se le ha abierto canal, y se lo
/// dice por el suyo como cualquier otro rechazo (ID-248). Atender dos a la vez
/// es meter a la persona en dos trámites de dos sedes con dos PIN a medias.
///
/// **Quien decide eso es [`LiveErrand::begin`], y nadie más.** No se pregunta
/// primero y se apunta después: preguntar y apuntar son dos tomas del candado,
/// y entre ellas cabe otra invocación —el enlace profundo y la instancia única
/// son dos caminos distintos hasta aquí (#357, #362)—, así que las dos podrían
/// verlo libre y quedar las dos servidas con un solo trámite apuntado. `begin`
/// mira y apunta bajo el mismo candado y devuelve si la plaza era suya; el
/// canal que ya se había abierto para la segunda se cierra y lo que la sede
/// recibe es su `SAF_45`.
pub fn attend_launch(url: &str, transport: ChannelTransport<'_>, live: &LiveErrand) -> Attendance {
    let url = match AfirmaUrl::parse(url) {
        Ok(url) => url,
        // Ni siquiera es una URL del protocolo: no hay `ports` que leer, así
        // que no hay socket que abrir.
        Err(refusal) => return Attendance::RefusingInTheWindow(refusal),
    };

    match LaunchRequest::from_url(&url) {
        Ok(request) => {
            let duty = ChannelDuty::Serve(request.credential().clone());
            match transport(request.ports(), duty) {
                Ok(channel) => {
                    let errand = Errand::of(request.credential().clone(), channel.port());
                    if live.begin(errand.clone()) {
                        return Attendance::Serving { channel, errand };
                    }

                    // La plaza era de otra sede. El canal recién abierto se
                    // cierra **por su asa** —soltarlo sin más no la ejecuta:
                    // `Shutdown` es un `FnOnce` en una caja y dejar caer la
                    // caja no llama a nadie—, y sólo después se le contesta
                    // como a cualquier otro rechazo del ID-248, volviendo a
                    // atar uno de los puertos que sorteó esta.
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

/// Contesta el rechazo por donde se pueda: por el socket si la sede sorteó
/// puertos, y si no, por la ventana (ID-248).
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
        // Ningún puerto libre: el rechazo sigue siendo el de la invocación, y
        // ahora sólo cabe la ventana.
        Err(_) => Attendance::RefusingInTheWindow(refusal),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::channel::{Shutdown, Situation};
    use crate::protocol::{ChannelCredential, Parameter, SafCode};

    /// **Grada A**: el transporte doblado por un cierre. Aquí no se ata ningún
    /// puerto ni se abre ningún socket (TD-52).
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

    /// Primera conducta: **la invocación buena abre el canal** en el primero de
    /// los puertos sorteados, sirviendo la conversación.
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

    /// Segunda conducta: **un rechazo con puertos se contesta por el socket**
    /// (ID-248).
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

    /// Tercera conducta: **sin `ports` sólo hay ventana**. Es el caso `v=3` del
    /// original, que asume la 3 justamente cuando `ports` falta.
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

    /// Una credencial mal formada se rechaza —nunca se ignora—, y como la sede
    /// sí sorteó puertos, el `SAF_03` sale por el socket.
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

    /// Lo que no es una URL del protocolo no trae puertos que leer.
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

    /// Con todos los puertos ocupados no hay canal, y una invocación buena se
    /// queda sin sitio donde hablar.
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

    /// Y un rechazo que tampoco puede atarse a nada vuelve a la ventana con el
    /// rechazo de la invocación, no con el del transporte.
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

    /// **rfirma no se ata jamás al 63117** (ID-215): los puertos que llegan al
    /// transporte son los de la URL y nada más, ni siquiera para contestar un
    /// error.
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
