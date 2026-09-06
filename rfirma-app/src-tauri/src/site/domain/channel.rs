//! El canal local visto desde dentro: su cometido, sus situaciones de error y el asa del abierto (ADR-0005, ADR-0009).

use std::fmt;

use super::protocol::{ChannelCredential, WireAnswer};

/// Cometido con el que se abrió el canal local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelDuty {
    /// Servir la conversación con la credencial acordada.
    Serve(ChannelCredential),
    /// Contestar un rechazo al primer mensaje y cerrar.
    Refuse(WireAnswer),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Ninguno de los puertos sorteados por la sede estaba libre.
    NoDrawnPortIsFree,
    /// El material criptográfico no puede utilizarse para la conexión TLS.
    MaterialNotUsable,
    /// Error del sistema al intentar iniciar la escucha en el socket.
    NotListening,
}

/// Fallo del canal compuesto por situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelError {
    situation: Situation,
    detail: String,
}

impl ChannelError {
    /// Crea un nuevo fallo con situación y detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
        }
    }

    /// Situación clasificada del error.
    pub fn situation(&self) -> Situation {
        self.situation
    }

    /// Detalle técnico del error.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for ChannelError {}

/// Canal abierto con su puerto de escucha y asa de cierre.
pub struct OpenChannel {
    port: u16,
    shutdown: Shutdown,
}

impl OpenChannel {
    /// Crea un canal abierto con su puerto y asa de cierre.
    pub fn new(port: u16, shutdown: Shutdown) -> Self {
        Self { port, shutdown }
    }

    /// Puerto en el que escucha el canal.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Cierra el canal y deja de escuchar conexiones.
    pub fn close(self) {
        self.shutdown.now();
    }
}

impl std::fmt::Debug for OpenChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenChannel")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// Asa para apagar el servidor del canal.
pub struct Shutdown(Box<dyn FnOnce() + Send>);

impl Shutdown {
    /// Construye un asa de apagado a partir de una clausura.
    pub fn of(closing: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(closing))
    }

    /// Ejecuta el apagado del servidor.
    pub fn now(self) {
        (self.0)();
    }
}

#[cfg(test)]
mod tests;
