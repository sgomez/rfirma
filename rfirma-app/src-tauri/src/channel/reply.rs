//! Asa de respuesta única para operaciones pendientes de la sede.

use tokio::sync::oneshot;

/// Asa de respuesta hacia la sede que se consume en un único envío.
pub struct ReplyHandle(oneshot::Sender<String>);

impl ReplyHandle {
    /// Envuelve el canal emisor en un asa de respuesta.
    pub fn of(sender: oneshot::Sender<String>) -> Self {
        Self(sender)
    }

    /// Contesta a la sede y cierra el canal.
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
mod tests;
