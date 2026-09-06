//! **Los dos puertos del trámite** (RD-03, RD-04): el códec del protocolo y
//! el transporte.
//!
//! El trámite no sabe cómo se escribe una petición en el cable ni por dónde
//! entra. Lo primero se lo dice el [`ProtocolCodec`] —de mensaje crudo a
//! [`SiteRequest`], y de [`SiteOutcome`] a la línea que sale— y lo segundo el
//! [`Transport`], que abre el canal por el que llegan los mensajes y ofrece el
//! [`ReplyHandle`] por el que se contesta mucho después.
//!
//! Hoy hay **un adaptador de cada uno**, y ninguno vive aquí: el códec de la
//! versión 4 es [`crate::app::codec::V4Codec`] y el transporte es el `wss`
//! sobre el *loopback* con puerto sorteado, [`crate::app::transport`]. Cuál de
//! los dos se instancia lo decide la negociación de arranque
//! ([`crate::app::site::negotiate`]) y nadie más (RD-05). El trámite no
//! nombra a ninguno de los dos: la guarda de dirección
//! (`tests/module_directions.rs`) pone en rojo la arista (RD-12).
//!
//! **No hay nada aquí por si acaso** (RD-10): la forma de los dos puertos es
//! todo lo que se deja para el futuro.

use std::sync::Arc;

use crate::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::protocol::AfirmaUrl;

use super::outcome::SiteOutcome;
use super::request::SiteRequest;

/// **El códec del protocolo** (RD-03): de lo que llega a lo que la sede quiere,
/// y de lo que el trámite produjo a lo que sale al cable.
///
/// El mensaje crudo es la URL `afirma://` ya partida en verbo y pares
/// ([`AfirmaUrl`]): es lo que entrega la conversación del canal una vez
/// pasadas sus guardias, y lo único que todas las versiones del protocolo
/// comparten.
pub trait ProtocolCodec {
    /// Lee la operación que llegó por el canal ya abierto.
    ///
    /// Nunca falla: una operación que no se atiende **es** una petición, la de
    /// contestarle a la sede por qué ([`SiteRequest::NotAttended`]).
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest;

    /// La línea exacta que se escribe en el canal para ese desenlace.
    fn encode(&self, outcome: &SiteOutcome) -> String;
}

/// **El asa por la que se le contesta a la sede** (ID-321, ID-322).
///
/// Una operación de sede no tiene respuesta en el momento en que llega: entre
/// el mensaje y su respuesta hay una persona que tiene que consentir. El
/// transporte se queda con la conexión abierta y entrega este asa al trámite;
/// quien acabe el trámite escribe por ella **una vez** y el canal se cierra.
///
/// Es parte del puerto y no un tipo del canal (RD-04): el trámite la guarda
/// ([`super::LiveErrand::answer_through`]) sin nombrar al transporte concreto.
/// Es un cierre y no el `oneshot` del servidor por lo mismo que el asa de
/// apagado del canal: para que ni el trámite ni sus dobles tengan que nombrar
/// a `tokio`.
///
/// # Contestar dos veces no existe
///
/// [`ReplyHandle::answer`] **consume** el asa, y el trámite la guarda en un
/// `Option` que se vacía al usarla: la segunda salida —cancelar después de
/// haber contestado, o cerrar la ventana con la sede ya servida— no escribe
/// nada porque no hay asa (ID-340). Que la sede ya no esté al otro lado no es
/// un fallo del trámite (ID-323): el desenlace se enseña igual en la ventana y
/// aquí no se reintenta nada.
pub struct ReplyHandle(Box<dyn FnOnce(String) + Send>);

impl ReplyHandle {
    /// El asa que entrega el texto por ese camino.
    pub fn of(deliver: impl FnOnce(String) + Send + 'static) -> Self {
        Self(Box::new(deliver))
    }

    /// Contesta esto a la sede y cierra el canal.
    ///
    /// No devuelve nada a propósito: que la conexión se haya caído mientras la
    /// persona decidía no cambia en nada lo que el trámite hace después
    /// (ID-323).
    pub fn answer(self, text: String) {
        (self.0)(text);
    }
}

impl std::fmt::Debug for ReplyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReplyHandle")
    }
}

/// **Quién atiende lo que entra por el transporte** (ID-330): la operación y el
/// asa por la que se le contestará.
///
/// Es `Arc` y no una referencia porque el transporte atiende cada conexión en
/// su propia tarea, que vive más que la llamada que abrió el canal.
pub type Inbox = Arc<dyn Fn(AfirmaUrl, ReplyHandle) + Send + Sync>;

/// **El transporte** (RD-04, ID-214): por dónde entra el mensaje y por dónde
/// sale la respuesta.
///
/// Recibe los puertos de la invocación negociada y el cometido con el que se
/// abre el canal —servir la conversación, o contestar un rechazo al primer
/// mensaje (ID-248)—, y devuelve el canal abierto con su asa de apagado. Los
/// mensajes que lleguen los entrega por el [`Inbox`] con el que se construyó
/// el adaptador.
pub trait Transport {
    /// Ata uno de esos puertos y sirve el canal para ese cometido.
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError>;
}

/// Un cierre con la misma firma **es** un transporte: es lo que permite doblar
/// el puerto con una lambda en las pruebas (TD-51, TD-52) y lo que hace que la
/// grada C del canal le pase al arranque el suyo sin nombrar este rasgo.
impl<F> Transport for F
where
    F: Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>,
{
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
        self(ports, duty)
    }
}

/// **El transporte tal y como lo reciben los casos de uso** (ID-214, ID-326):
/// una referencia al cierre, que es la forma en la que una prueba lo dobla sin
/// anotar tipos y en la que el arranque de Tauri le pasa el de producción.
pub type ChannelTransport<'a> =
    &'a dyn Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Lo que se contesta por el asa es lo que recibe el otro extremo, y sólo
    /// se contesta una vez porque el asa se consume.
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

    /// Un cierre es un transporte: el que dobla las pruebas y el que el canal
    /// de la grada C le pasa al arranque.
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
