//! Puertos del contexto de sede: códec, transporte, almacenes de confianza y ranuras de la CA local (ADR-0017).

use std::path::Path;
use std::sync::Arc;

use crate::site::domain::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::protocol::AfirmaUrl;
use crate::site::domain::tls_error::TlsError;
use crate::site::domain::trust_error::TrustError;

use crate::site::application::errand::outcome::SiteOutcome;
use crate::site::application::errand::request::SiteRequest;

/// Códec del protocolo para decodificar peticiones y codificar desenlaces.
pub trait ProtocolCodec {
    /// Lee la operación que llegó por el canal abierto.
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest;

    /// Línea exacta que se escribe en el canal para el desenlace dado.
    fn encode(&self, outcome: &SiteOutcome) -> String;
}

/// Asa de respuesta única para contestar a la sede y cerrar el canal.
pub struct ReplyHandle(Box<dyn FnOnce(String) + Send>);

impl ReplyHandle {
    /// Crea un asa con la función de entrega dada.
    pub fn of(deliver: impl FnOnce(String) + Send + 'static) -> Self {
        Self(Box::new(deliver))
    }

    /// Contesta a la sede y consume el asa.
    pub fn answer(self, text: String) {
        (self.0)(text);
    }
}

impl std::fmt::Debug for ReplyHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReplyHandle")
    }
}

/// Receptor de operaciones entrantes y sus asas de respuesta.
pub type Inbox = Arc<dyn Fn(AfirmaUrl, ReplyHandle) + Send + Sync>;

/// Puerto de transporte para abrir canales de comunicación.
pub trait Transport {
    /// Abre un canal en los puertos indicados para el cometido especificado.
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError>;
}

impl<F> Transport for F
where
    F: Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>,
{
    fn open(&self, ports: &[u16], duty: ChannelDuty) -> Result<OpenChannel, ChannelError> {
        self(ports, duty)
    }
}

/// Referencia al transporte para los casos de uso.
pub type ChannelTransport<'a> =
    &'a dyn Fn(&[u16], ChannelDuty) -> Result<OpenChannel, ChannelError>;

/// Puerto de interacción con los almacenes NSS (ADR-0005).
pub trait TrustStores {
    /// Instala el certificado en el almacén de perfil indicado con permisos de confianza TLS.
    fn install(
        &self,
        profile: &Path,
        certificate_der: &[u8],
        nickname: &str,
    ) -> Result<(), TrustError>;

    /// Obtiene los bits de confianza TLS configurados para el certificado en el almacén.
    fn trust_of(&self, profile: &Path, certificate_der: &[u8]) -> Result<Option<u32>, TrustError>;
}

/// Las dos ranuras de la CA local: la que sirve y la siguiente del solape (ADR-0005).
pub trait LocalCaSlots {
    /// La CA local que sirve, si la hay.
    fn serving(&self) -> Result<Option<LocalCa>, TlsError>;

    /// Guarda la CA local que sirve sustituyendo la anterior.
    fn write_serving(&self, ca: &LocalCa) -> Result<(), TlsError>;

    /// La CA local siguiente, si la hay.
    fn next(&self) -> Result<Option<LocalCa>, TlsError>;

    /// Guarda la CA local siguiente sin tocar la que sirve.
    fn write_next(&self, ca: &LocalCa) -> Result<(), TlsError>;

    /// Promueve la siguiente a la que sirve y vacía su ranura.
    fn promote_next(&self) -> Result<Option<LocalCa>, TlsError>;

    /// Vacía la ranura de la siguiente.
    fn forget_next(&self) -> Result<(), TlsError>;
}

#[cfg(test)]
mod tests;
