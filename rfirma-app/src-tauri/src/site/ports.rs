//! Puertos del contexto de sede: transporte, confianza, CA local, los dos motores del puente y lo que el trámite pide a los vecinos (ADR-0017).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::identity::domain::certificate::{ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::BridgeError;
use crate::site::domain::channel::{ChannelDuty, ChannelError, OpenChannel};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::protocol::AfirmaUrl;
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::domain::tls_error::TlsError;
use crate::site::domain::trust_error::TrustError;

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

/// El motor de filtros que presta el puente: qué certificados cumplen la expresión de la sede.
pub trait FilterEngine {
    /// Devuelve los índices de los certificados que cumplen los criterios.
    fn select(
        &self,
        filter_properties: &str,
        certificates_b64: &str,
    ) -> Result<Vec<usize>, BridgeError>;
}

/// El expansor de la política de firma declarada por la sede, que también presta el puente.
pub trait PolicyEngine {
    /// Expande las propiedades de política de firma en formato Java Properties.
    fn expand(&self, extra_params: &str, format: &str) -> Result<String, BridgeError>;
}

/// Los certificados de la persona vistos desde el trámite: los que hay, sus filas con asa y el que está tras un asa.
pub trait Certificates {
    /// Los certificados de todos los almacenes, o por qué ninguno se ha podido abrir.
    fn listed(&self) -> Result<Vec<TokenCertificate>, TokenError>;

    /// Las filas con su asa acuñada y el recordado marcado, para la ventana.
    fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate>;

    /// El certificado de la última búsqueda tras el asa, si sigue en el token y está vigente.
    fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError>;
}

/// El documento de paso del trámite, apuntado como abierto sin rastro para que la ventana lo lea (ADR-0011).
pub trait ScratchDocuments {
    /// Apunta el documento y devuelve el asa con la que la ventana lo pide.
    fn open_unrecorded(&self, path: PathBuf) -> String;
}

/// La carpeta de paso del trámite: donde cae el documento de la sede y de donde se borra (ADR-0016).
pub trait Scratch {
    /// Se asegura de que la carpeta de paso existe.
    fn make_the_folder(&self, folder: &Path) -> Result<(), String>;

    /// Deja los bytes del documento en esa ruta.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;

    /// Borra el fichero de paso, y calla si ya no estaba.
    fn erase(&self, path: &Path);
}

/// Lo que el trámite pide firmar: el documento por su asa, el certificado ya cribado y lo que declaró la sede.
#[derive(Clone, Copy, Debug)]
pub struct SiteSigningRequest<'a> {
    /// El asa del documento de paso.
    pub document: &'a str,
    /// El certificado que la persona eligió y la sede acepta.
    pub certificate: &'a TokenCertificate,
    /// Los parámetros de la sede, ya expandidos.
    pub from_the_site: &'a BTreeMap<String, String>,
    /// Si la sede consintió cofirmar sobre firmas que no se reconocen.
    pub allow_unregistered_signatures: bool,
}

/// La firma que la sede pide, hecha por quien firma; lo que sale mal vuelve ya con su código y su vista.
pub trait SiteSigning {
    /// Abre el ciclo y dice cómo se pide el secreto.
    fn begin(&self, request: SiteSigningRequest<'_>) -> Result<StoreSecret, SigningRefusal>;

    /// Cierra el ciclo en memoria: el PDF firmado y el DER del firmante, sin escribir nada.
    fn finish(&self) -> Result<SiteSignature, SigningRefusal>;
}

#[cfg(test)]
mod tests;
