//! Gestión del ciclo de vida y retención de canales de comunicación abiertos con la sede.

use crate::channel::OpenChannel;

use super::super::site::Attendance;

/// Contenedor en memoria de los canales de comunicación activos para trámites o rechazos.
#[derive(Default)]
pub struct HeldChannel {
    /// Canal asignado al trámite en curso.
    serving: std::sync::Mutex<Option<OpenChannel>>,
    /// Canal temporal para emitir un código de rechazo por socket.
    refusing: std::sync::Mutex<Option<OpenChannel>>,
}

impl HeldChannel {
    /// Almacena el canal de un trámite activo cerrando el anterior si existía.
    pub fn hold(&self, channel: OpenChannel) {
        if let Some(previous) = super::super::lock(&self.serving).replace(channel) {
            previous.close();
        }
    }

    /// Comprueba si existe un canal de trámite activo en servicio.
    pub fn is_serving(&self) -> bool {
        super::super::lock(&self.serving).is_some()
    }

    /// Almacena un canal temporal para comunicar un rechazo por socket.
    pub fn hold_a_refusal(&self, channel: OpenChannel) {
        if let Some(previous) = super::super::lock(&self.refusing).replace(channel) {
            previous.close();
        }
    }
}

/// Registra el canal resultante de la atención de sede o emite el aviso correspondiente.
pub fn hold_the_channel(held: &HeldChannel, attendance: Attendance) -> Vec<String> {
    match attendance {
        Attendance::Serving { channel, .. } => {
            held.hold(channel);
            Vec::new()
        }
        Attendance::RefusingOverTheChannel { channel, .. } => {
            held.hold_a_refusal(channel);
            Vec::new()
        }
        Attendance::RefusingInTheWindow(refusal) => vec![format!(
            "rfirma: la invocacion de sede se rechaza con {} y no hay canal por el que decirlo: {}",
            refusal.answer().on_the_wire(),
            refusal.detail()
        )],
        Attendance::ChannelNotOpened(error) => vec![format!(
            "rfirma: la invocacion de sede era buena pero no se abrio el canal ({error})"
        )],
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::channel::Shutdown;

    const PORTS: [u16; 3] = [51001, 51002, 51003];

    fn a_channel(port: u16, closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> OpenChannel {
        let closed = std::sync::Arc::clone(closed);
        OpenChannel::new(
            port,
            Shutdown::of(move || crate::app::lock(&closed).push(port)),
        )
    }

    fn closed_ports(closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> Vec<u16> {
        crate::app::lock(closed).clone()
    }

    #[test]
    fn a_refusal_never_closes_the_channel_of_the_live_errand() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold(a_channel(PORTS[0], &closed));
        held.hold_a_refusal(a_channel(PORTS[1], &closed));

        assert!(
            closed_ports(&closed).is_empty(),
            "el canal del trámite vivo sigue sirviendo: {:?}",
            closed_ports(&closed)
        );
    }

    #[test]
    fn a_new_refusal_closes_the_refusal_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold_a_refusal(a_channel(PORTS[0], &closed));
        held.hold_a_refusal(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }

    #[test]
    fn an_unheld_channel_is_not_serving() {
        let held = HeldChannel::default();

        assert!(!held.is_serving());
    }

    #[test]
    fn only_the_channel_of_the_errand_counts_as_serving() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold_a_refusal(a_channel(PORTS[0], &closed));
        assert!(!held.is_serving());

        held.hold(a_channel(PORTS[1], &closed));
        assert!(held.is_serving());
    }

    #[test]
    fn a_new_serving_channel_closes_the_one_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold(a_channel(PORTS[0], &closed));
        held.hold(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }
}
