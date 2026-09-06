//! **El canal abierto, sostenido mientras haga falta** (ID-325).
//!
//! No es una decisión, es una consecuencia: soltar un [`OpenChannel`] suelta
//! con él su asa de apagado —el emisor del `oneshot` que espera el servidor—, y
//! la tarea que acepta conexiones termina. Sin alguien que lo guarde, el canal
//! que acaba de abrirse se cierra en cuanto el arranque devuelve, y la sede se
//! queda esperando exactamente igual que en el #390.

use crate::channel::OpenChannel;

use super::super::site::Attendance;

/// **Las dos ranuras del canal abierto.**
///
/// Vive en el estado de Tauri, como el trámite (ID-325).
///
/// # Dos ranuras, y la razón es el ID-280
///
/// Un canal de rechazo (`SAF_45` y cualquier otro del ID-248) **no puede
/// compartir ranura con el del trámite**: cuando llega una segunda invocación
/// con un trámite ya vivo, [`site::attend_launch`] abre un canal nuevo sólo
/// para decir el código, y meterlo donde estaba el del trámite cerraría
/// justamente el canal que está sirviendo al primero —el que llega dejaría
/// fuera al que estaba, que es lo contrario del criterio (ID-279, ID-280) y el
/// síntoma mismo del #390—.
///
/// Así que el que sirve y el que rechaza se guardan aparte: `hold` es del
/// trámite y `hold_a_refusal` del rechazo, y ninguno toca la ranura del otro.
#[derive(Default)]
pub struct HeldChannel {
    /// El canal del trámite que se quedó con la plaza.
    serving: std::sync::Mutex<Option<OpenChannel>>,
    /// El canal abierto sólo para contestar un rechazo por el socket (ID-248).
    refusing: std::sync::Mutex<Option<OpenChannel>>,
}

impl HeldChannel {
    /// Se queda con el canal **del trámite**. El que hubiera sirviendo **se
    /// cierra**: sólo hay un trámite a la vez (ID-280), y si hay uno nuevo
    /// sirviendo es que el anterior terminó y ya no tiene quien lo conteste.
    pub fn hold(&self, channel: OpenChannel) {
        if let Some(previous) = super::super::lock(&self.serving).replace(channel) {
            previous.close();
        }
    }

    /// **¿Hay canal sirviendo al trámite?**
    ///
    /// La pregunta que hay que hacerse antes de mandar a la ventana de sede a
    /// esperar: esperar sólo tiene sentido si queda alguien escuchando la
    /// petición del navegador. Mira **la ranura del trámite y sólo ésa**: un
    /// canal de rechazo (ID-248) vive lo justo para decir su código y no
    /// atiende a nadie más.
    ///
    /// No dice nada de la CA local ni de los almacenes NSS: eso lo contesta
    /// [`crate::app::trust::refresh_local_ca_trust`], y confundir las dos
    /// preguntas es lo que hacía que la orden 36 publicase «Conectando con la
    /// sede» sobre un canal que nunca se abrió.
    pub fn is_serving(&self) -> bool {
        super::super::lock(&self.serving).is_some()
    }

    /// Sostiene el canal de un **rechazo** mientras contesta (ID-248).
    ///
    /// Vive lo justo para decir su código: no se le suelta en el acto porque
    /// soltarlo apaga el servidor antes de que la sede llegue a conectarse, y
    /// no se cierra a mano porque nadie sabe aquí cuándo ha contestado. Lo
    /// cierra el rechazo siguiente, y si no llega ninguno, el fin del proceso.
    ///
    /// **Nunca toca el canal del trámite vivo**: un rechazo es exactamente el
    /// caso en el que el anterior sí tiene quien lo conteste.
    pub fn hold_a_refusal(&self, channel: OpenChannel) {
        if let Some(previous) = super::super::lock(&self.refusing).replace(channel) {
            previous.close();
        }
    }
}

/// **Caso de uso.** Sostiene el canal que se acaba de abrir, o cuenta por qué
/// no lo hay (ID-325, ID-341).
///
/// Un rechazo que no tiene socket por el que salir **ya se ha enseñado en la
/// ventana de sede**; lo que se devuelve es la línea del registro, que es para
/// quien lee `stderr` y no para quien está delante.
pub fn hold_the_channel(held: &HeldChannel, attendance: Attendance) -> Vec<String> {
    match attendance {
        Attendance::Serving { channel, .. } => {
            held.hold(channel);
            Vec::new()
        }
        // **Por su propia ranura, nunca por la del trámite** (ID-279, ID-280):
        // con un trámite ya vivo el canal que llega es el del `SAF_45`, y
        // guardarlo donde el que sirve cerraría el del primero —el que llega
        // dejaría fuera al que estaba, y la sede del trámite en marcha se
        // quedaría esperando igual que en el #390—.
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

    /// Los puertos que sortea la sede en las pruebas.
    const PORTS: [u16; 3] = [51001, 51002, 51003];

    /// Un canal que apunta su cierre: es lo único que hace falta para ver a
    /// [`HeldChannel`] por dentro, porque cerrar es lo único que hace.
    fn a_channel(port: u16, closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> OpenChannel {
        let closed = std::sync::Arc::clone(closed);
        OpenChannel::new(
            port,
            Shutdown::of(move || crate::app::lock(&closed).push(port)),
        )
    }

    /// Qué puertos se han cerrado hasta ahora.
    fn closed_ports(closed: &std::sync::Arc<Mutex<Vec<u16>>>) -> Vec<u16> {
        crate::app::lock(closed).clone()
    }

    /// **ID-279, ID-280.** Sostener el canal de un rechazo **no cierra el del
    /// trámite vivo**: con un trámite en marcha, el que llega se queda fuera, y
    /// eso es exactamente lo contrario de que el que llega eche al que estaba.
    ///
    /// Es la mitad que no ve
    /// [`a_second_launch_with_a_live_errand_gets_no_window_of_its_own`]: ésa es
    /// de grada A sobre el caso de uso y no llega hasta la ranura.
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

    /// Un rechazo detrás de otro sí cierra al anterior: el primero ya contestó
    /// lo suyo, y su puerto no tiene por qué seguir atado.
    #[test]
    fn a_new_refusal_closes_the_refusal_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold_a_refusal(a_channel(PORTS[0], &closed));
        held.hold_a_refusal(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }

    /// Sin canal del trámite no hay a quién esperar, y la ranura lo dice.
    ///
    /// Es lo que separa las dos situaciones desde las que se llega al botón de
    /// instalar la CA local: con el canal en pie la petición de la sede puede
    /// llegar todavía, y sin él la pantalla de reparación es la respuesta.
    #[test]
    fn an_unheld_channel_is_not_serving() {
        let held = HeldChannel::default();

        assert!(!held.is_serving());
    }

    /// Un canal de rechazo **no** es un canal sirviendo: vive lo justo para
    /// decir su código (ID-248) y nadie va a atender por él la petición.
    #[test]
    fn only_the_channel_of_the_errand_counts_as_serving() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold_a_refusal(a_channel(PORTS[0], &closed));
        assert!(!held.is_serving());

        held.hold(a_channel(PORTS[1], &closed));
        assert!(held.is_serving());
    }

    /// **ID-280.** Y un trámite nuevo sí cierra el canal del anterior: si hay
    /// otro sirviendo es que el primero terminó.
    #[test]
    fn a_new_serving_channel_closes_the_one_it_replaces() {
        let closed = std::sync::Arc::new(Mutex::new(Vec::new()));
        let held = HeldChannel::default();

        held.hold(a_channel(PORTS[0], &closed));
        held.hold(a_channel(PORTS[1], &closed));

        assert_eq!(closed_ports(&closed), vec![PORTS[0]]);
    }
}
