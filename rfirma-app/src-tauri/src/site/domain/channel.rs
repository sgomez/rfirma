//! El canal local visto desde dentro: su cometido, sus situaciones de error y el asa del abierto (ADR-0005, ADR-0009).

use std::fmt;

use super::protocol::{NegotiatedCredential, Refusal, RelayChannelInfo, WireAnswer};

/// Cometido con el que se abrió el canal local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelDuty {
    /// Servir la conversación con la credencial negociada, si la hay.
    Serve(NegotiatedCredential),
    /// Contestar un rechazo al primer mensaje y cerrar.
    Refuse(WireAnswer),
}

/// Dónde escucha el canal: los puertos que sorteó la sede, probados en el orden en que los
/// mandó, o un puerto fijo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelLocation {
    /// Puertos sorteados por la sede.
    Drawn(Vec<u16>),
    /// Puerto fijo, atado tal cual.
    Fixed(u16),
    /// Puertos sorteados por la sede para el transporte `service`: no caben en `Drawn`, que ya
    /// significa wss.
    Service(Vec<u16>),
    /// Servidor intermedio: sin puerto que escuchar, la operación viaja con sus servlets.
    Relay(RelayChannelInfo),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    /// Ninguno de los puertos sorteados por la sede estaba libre.
    NoDrawnPortIsFree,
    /// El material criptográfico no puede utilizarse para la conexión TLS.
    MaterialNotUsable,
    /// Error del sistema al intentar iniciar la escucha en el socket.
    NotListening,
    /// El servidor intermedio no pudo completar la operación; el detalle va en `refusal()`.
    Relay,
}

/// Fallo del canal compuesto por situación clasificada y detalle técnico.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelError {
    situation: Situation,
    detail: String,
    refusal: Option<Refusal>,
}

impl ChannelError {
    /// Crea un nuevo fallo con situación y detalle técnico.
    pub fn new(situation: Situation, detail: impl Into<String>) -> Self {
        Self {
            situation,
            detail: detail.into(),
            refusal: None,
        }
    }

    /// Un fallo del servidor intermedio, ya clasificado con su `SAF_NN` y su detalle (ADR-0009).
    pub fn refused(refusal: Refusal) -> Self {
        Self {
            situation: Situation::Relay,
            detail: refusal.detail().to_owned(),
            refusal: Some(refusal),
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

    /// El rechazo ya clasificado, cuando este error lo trae (servidor intermedio).
    pub fn refusal(&self) -> Option<&Refusal> {
        self.refusal.as_ref()
    }
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.situation, self.detail)
    }
}

impl std::error::Error for ChannelError {}

/// Canal abierto con su puerto de escucha, asa de cierre y, si la trae, la entrega ya resuelta.
pub struct OpenChannel {
    port: u16,
    shutdown: Shutdown,
    delivery: Option<Delivery>,
}

impl OpenChannel {
    /// Crea un canal abierto con su puerto y asa de cierre, sin entrega pendiente.
    pub fn new(port: u16, shutdown: Shutdown) -> Self {
        Self {
            port,
            shutdown,
            delivery: None,
        }
    }

    /// Un canal que, además, trae ya resuelta la operación a entregar: quien lo abre no espera a
    /// un mensaje futuro (servidor intermedio), así que la entrega no puede correr hasta que
    /// quien llama haya registrado el trámite, o se atendería sin él (ADR-0016).
    pub fn with_delivery(port: u16, shutdown: Shutdown, delivery: Delivery) -> Self {
        Self {
            port,
            shutdown,
            delivery: Some(delivery),
        }
    }

    /// Puerto en el que escucha el canal.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Retira la entrega pendiente, si la trae, para que quien la retira decida cuándo dispararla.
    pub fn take_delivery(&mut self) -> Option<Delivery> {
        self.delivery.take()
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

/// Entrega diferida de una operación ya resuelta al abrir el canal.
pub struct Delivery(Box<dyn FnOnce() + Send>);

impl Delivery {
    /// Construye una entrega a partir de una clausura.
    pub fn of(delivering: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(delivering))
    }

    /// Dispara la entrega.
    pub fn now(self) {
        (self.0)();
    }
}

#[cfg(test)]
mod tests;
