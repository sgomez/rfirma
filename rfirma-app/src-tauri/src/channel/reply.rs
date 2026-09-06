//! **El asa por la que se le contesta a la sede** (ID-321, ID-322).
//!
//! Una operación de sede no tiene respuesta en el momento en que llega: entre
//! el mensaje y su respuesta hay una persona que tiene que consentir. El
//! servidor se queda con la conexión abierta ([`Answer::Pending`]) y entrega
//! este asa al trámite; quien acabe el trámite escribe por ella **una vez** y
//! el canal se cierra.
//!
//! [`Answer::Pending`]: super::conversation::Answer::Pending
//!
//! # Por qué es un `oneshot` y no el socket
//!
//! Quien contesta es un caso de uso de [`crate::app::errand`], llamado desde
//! una orden de la ventana, y no puede tocar el socket: está dentro de una
//! tarea del runtime que atiende esa conexión. Lo que cruza es el texto, por un
//! canal de una sola entrega; el que escribe de verdad sigue siendo
//! [`super::server`].
//!
//! # Contestar dos veces no existe
//!
//! [`ReplyHandle::answer`] **consume** el asa, y el trámite la guarda en un
//! `Option` que se vacía al usarla ([`crate::app::errand::LiveErrand`]): la
//! segunda salida —cancelar después de haber contestado, o cerrar la ventana
//! con la sede ya servida— no escribe nada porque no hay asa (ID-340).

use tokio::sync::oneshot;

/// El asa de respuesta de un trámite: se escribe por ella una vez y se acaba.
///
/// Que la sede ya no esté al otro lado no es un fallo del trámite (ID-323): el
/// desenlace se enseña igual en la ventana y aquí no se reintenta nada.
pub struct ReplyHandle(oneshot::Sender<String>);

impl ReplyHandle {
    /// El asa que entrega el texto por ese canal de una sola entrega.
    pub fn of(sender: oneshot::Sender<String>) -> Self {
        Self(sender)
    }

    /// Contesta esto a la sede y cierra el canal.
    ///
    /// No devuelve nada a propósito: que la conexión se haya caído mientras la
    /// persona decidía no cambia en nada lo que el trámite hace después
    /// (ID-323).
    pub fn answer(self, text: String) {
        let _ = self.0.send(text);
    }
}

impl std::fmt::Debug for ReplyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReplyHandle")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_answered_is_what_the_other_end_receives() {
        let (sender, mut receiver) = oneshot::channel();

        ReplyHandle::of(sender).answer("OK".to_owned());

        assert_eq!(receiver.try_recv(), Ok("OK".to_owned()));
    }

    /// Una conexión que se cayó con la operación pendiente **no tumba el
    /// trámite** (ID-323): se contesta igual, y nadie se entera de nada.
    #[test]
    fn answering_a_connection_that_is_gone_is_not_a_failure() {
        let (sender, receiver) = oneshot::channel();
        drop(receiver);

        ReplyHandle::of(sender).answer("CANCEL".to_owned());
    }
}
