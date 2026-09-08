//! Puertos del contexto de sede: transporte, confianza, CA local, servlets del servidor intermedio, los dos motores del puente y lo que el trámite pide a los vecinos (ADR-0017).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::identity::domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::bridge::{
    BridgeError, Format, SignatureOperation, XadesVariant, XmlDsigVariant,
};
use crate::site::domain::batch::{BatchFormat, TriphaseData};
use crate::site::domain::batch_error::BatchError;
use crate::site::domain::channel::{ChannelDuty, ChannelError, ChannelLocation, OpenChannel};
use crate::site::domain::local_ca::LocalCa;
use crate::site::domain::protocol::{
    AfirmaUrl, AskedAlgorithm, RequestedFormat, SignatureRound, XadesEnvelope, XmlDsigEnvelope,
};
use crate::site::domain::relay_error::RelayError;
use crate::site::domain::signing::{SigningRefusal, SiteSignature};
use crate::site::domain::tls_error::TlsError;
use crate::site::domain::trust_error::TrustError;

impl From<SignatureRound> for SignatureOperation {
    fn from(round: SignatureRound) -> Self {
        match round {
            SignatureRound::First => Self::Sign,
            SignatureRound::Again => Self::Cosign,
            SignatureRound::Counter { .. } => Self::Countersign,
        }
    }
}

impl From<RequestedFormat> for Format {
    fn from(requested: RequestedFormat) -> Self {
        match requested {
            RequestedFormat::Pades => Self::Pades,
            RequestedFormat::Cades => Self::Cades,
            RequestedFormat::CadesAsicS => Self::CadesAsicS,
            RequestedFormat::Cms => Self::Cms,
            RequestedFormat::Xades(XadesEnvelope::Detached) => Self::Xades(XadesVariant::Detached),
            RequestedFormat::Xades(XadesEnvelope::Enveloping) => {
                Self::Xades(XadesVariant::Enveloping)
            }
            RequestedFormat::Xades(XadesEnvelope::Enveloped) => {
                Self::Xades(XadesVariant::Enveloped)
            }
            RequestedFormat::Xades(XadesEnvelope::AsicS) => Self::Xades(XadesVariant::AsicS),
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Detached) => {
                Self::XmlDsig(XmlDsigVariant::Detached)
            }
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Enveloping) => {
                Self::XmlDsig(XmlDsigVariant::Enveloping)
            }
            RequestedFormat::XmlDsig(XmlDsigEnvelope::Enveloped) => {
                Self::XmlDsig(XmlDsigVariant::Enveloped)
            }
            RequestedFormat::FacturaE => Self::FacturaE,
        }
    }
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
    /// Abre un canal en la ubicación indicada para el cometido especificado.
    fn open(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError>;
}

impl<F> Transport for F
where
    F: Fn(&ChannelLocation, ChannelDuty) -> Result<OpenChannel, ChannelError>,
{
    fn open(
        &self,
        location: &ChannelLocation,
        duty: ChannelDuty,
    ) -> Result<OpenChannel, ChannelError> {
        self(location, duty)
    }
}

/// Referencia al transporte para los casos de uso.
pub type ChannelTransport<'a> =
    &'a dyn Fn(&ChannelLocation, ChannelDuty) -> Result<OpenChannel, ChannelError>;

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

/// Puerto de salida hacia los servlets del servidor intermedio: recuperar, almacenar y esperar.
pub trait Servlets {
    /// Recupera los datos guardados bajo el identificador dado.
    fn retrieve(&self, service_url: &str, id: &str) -> Result<String, RelayError>;

    /// Almacena datos bajo el identificador dado.
    fn store(&self, service_url: &str, id: &str, data: &str) -> Result<(), RelayError>;

    /// Envía una señal de espera activa bajo el identificador dado (`requestWait` del original).
    fn wait(&self, service_url: &str, id: &str) -> Result<(), RelayError>;
}

/// Puerto de salida hacia los dos servlets del lote remoto: prefirma y postfirma (`BatchSigner`,
/// 1.9.2).
pub trait BatchServices {
    /// Prefirma el lote: manda el lote y la cadena de certificados, y recibe la respuesta cruda.
    fn presign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
    ) -> Result<Vec<u8>, BatchError>;

    /// Postfirma el lote: manda el lote, la cadena de certificados y el `TriphaseData` con los
    /// `PK1` ya calculados, y recibe la respuesta cruda.
    fn postsign(
        &self,
        url: &str,
        format: BatchFormat,
        lote_base64: &str,
        certs: &[Vec<u8>],
        tridata: &TriphaseData,
    ) -> Result<Vec<u8>, BatchError>;
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

    /// El certificado recordado entre sesiones, si lo hay.
    fn remembered(&self) -> Option<CertificateRef>;

    /// Apunta el certificado elegido para la próxima sede que lo pegue.
    fn remember(&self, chosen: &CertificateRef);

    /// Olvida el certificado recordado.
    fn forget_the_remembered(&self);
}

/// El documento de paso del trámite, apuntado como abierto sin rastro para que la ventana lo lea (ADR-0011).
pub trait ScratchDocuments {
    /// Apunta el documento y devuelve el asa con la que la ventana lo pide.
    fn open_unrecorded(&self, path: PathBuf) -> String;
}

/// El acceso a disco del trámite: la carpeta de paso del documento de la sede (ADR-0016) y las
/// rutas que la persona elige por el diálogo del portal al guardar o cargar (ADR-0011).
pub trait Scratch {
    /// Se asegura de que la carpeta de paso existe.
    fn make_the_folder(&self, folder: &Path) -> Result<(), String>;

    /// Deja los bytes del documento en esa ruta.
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;

    /// Lee los bytes de esa ruta.
    fn read(&self, path: &Path) -> Result<Vec<u8>, String>;

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
    /// El formato de firma que pidió la sede.
    pub format: Format,
    /// La huella que pidió la sede, todavía sin componer con la clave del certificado.
    pub algorithm: AskedAlgorithm,
    /// Qué pidió hacer la sede con el documento.
    pub operation: SignatureOperation,
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

/// La firma de bytes con el token que pide el lote remoto: el secreto se abre una vez y sirve para todas las firmas, sin puente y sin que la clave salga del token (ADR-0001).
pub trait TokenSigning {
    /// Cómo se pide el secreto del certificado, una sola vez para todas las firmas.
    fn secret_of(&self, certificate: &TokenCertificate) -> Result<StoreSecret, SigningRefusal>;

    /// Firma esos bytes con el algoritmo que declaró la sede y el secreto ya abierto.
    fn sign(
        &self,
        certificate: &TokenCertificate,
        secret: &str,
        algorithm: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, SigningRefusal>;
}

#[cfg(test)]
mod tests;
