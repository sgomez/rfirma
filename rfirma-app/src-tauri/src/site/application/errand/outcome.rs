//! Vocabulario de salida del trámite con la sede y la ventana, y el códec que lo pone en el cable.

use std::collections::BTreeMap;

use crate::identity::domain::certificate::ListedCertificate;
use crate::signing::domain::bridge::Format;
use crate::site::application::session::SiteRefusal;
use crate::site::domain::batch::LocalBatch;
use crate::site::domain::protocol::{
    AfirmaUrl, AskedAlgorithm, BatchRequest, Refusal, SignAndSaveRequest, SignatureRound,
    SiteFilter, SiteVisibleSignature,
};
use crate::site::domain::signing::SiteSignature;

use super::request::SiteRequest;

/// En qué queda la operación que llegó por el canal.
#[derive(Debug)]
pub enum ErrandStep {
    /// Momento de consentimiento de certificado para la ventana.
    AskingForConsent {
        /// Certificados aceptados por la sede, ya cribados.
        certificates: Vec<ListedCertificate>,
        /// Filtro solicitado por la sede para volver a comprobarlo (ADR-0011).
        filter: SiteFilter,
        /// Si la sede pidió recordar el certificado que se elija.
        sticky: bool,
    },
    /// Momento de consentimiento de firma de documento para la ventana.
    AskingToSign(SigningConsent),
    /// Momento de consentimiento del lote remoto para la ventana.
    AskingToSignTheBatch(Box<BatchConsent>),
    /// Momento de consentimiento del lote local para la ventana.
    AskingToSignTheLocalBatch(Box<LocalBatchConsent>),
    /// Paso de guardado: la orden de Tauri abre el diálogo del portal y escribe.
    Saving(Box<SavingConsent>),
    /// Paso de carga: la orden de Tauri abre el selector del portal y lee.
    Loading(LoadingConsent),
    /// Trámite sin ningún certificado con el que continuar.
    NoCertificate {
        /// Razón por la que no hay certificado.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona en su almacén.
        owned: usize,
        /// Lo que la sede ya ha recibido, si correspondía enviar algo.
        answered: Option<SiteOutcome>,
    },
    /// Respuesta directa para la sede sin requerir consentimiento.
    Answering(SiteOutcome),
}

impl ErrandStep {
    /// Momento que este paso deja delante de la persona, si deja alguno.
    pub fn moment(&self) -> Option<Moment> {
        match self {
            Self::AskingForConsent { certificates, .. } => Some(Moment::AskingForConsent {
                certificates: certificates.clone(),
            }),
            Self::AskingToSign(consent) => Some(Moment::AskingToSign {
                document: consent.document.clone(),
                format: consent.format,
                round: consent.round,
                certificates: consent.certificates.clone(),
                unregistered_signatures: consent.unregistered_signatures,
            }),
            Self::AskingToSignTheBatch(consent) => Some(Moment::AskingToSignTheBatch {
                signs: consent.signs,
                certificates: consent.certificates.clone(),
                already_chosen: consent.already_chosen.clone(),
            }),
            Self::AskingToSignTheLocalBatch(consent) => Some(Moment::AskingToSignTheLocalBatch {
                items: consent.items.clone(),
                certificates: consent.certificates.clone(),
                already_chosen: consent.already_chosen.clone(),
            }),
            Self::Saving(consent) => Some(Moment::Saving {
                filename: consent.filename.clone().or_else(|| consent.title.clone()),
            }),
            Self::Loading(consent) => Some(Moment::Loading {
                multiple: consent.multiple,
            }),
            Self::NoCertificate { reason, owned, .. } => Some(Moment::NoCertificate {
                reason: *reason,
                owned: *owned,
            }),
            Self::Answering(_) => None,
        }
    }
}

/// En qué queda el selector de carga tras elegir: el trámite sigue con un paso nuevo, o ya se ha
/// contestado a la sede con este desenlace.
#[derive(Debug)]
pub enum LoadCompletion {
    /// La sede sigue esperando: `signandsave` continúa con el documento ya elegido.
    Continues(ErrandStep),
    /// Ya se ha contestado a la sede.
    Delivered(SiteOutcome),
}

/// Motivo por el que no queda ningún certificado con el que seguir.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoCertificate {
    /// La persona no tiene certificados instalados.
    NotOne,
    /// La sede ha excluido todos los certificados instalados.
    TheSiteExcludedThemAll,
}

/// Datos necesarios para el consentimiento de firma.
#[derive(Debug)]
pub struct SigningConsent {
    /// Identificador del documento para la ventana (ADR-0011).
    pub document: String,
    /// Formato de firma que la sede pidió, ya atendido por el puente.
    pub format: Format,
    /// Huella que la sede pidió para esta firma.
    pub algorithm: AskedAlgorithm,
    /// Modalidad de firma solicitada.
    pub round: SignatureRound,
    /// Certificados aceptados por la sede, ya cribados.
    pub certificates: Vec<ListedCertificate>,
    /// Parámetros adicionales de la sede, ya expandidos.
    pub from_the_site: BTreeMap<String, String>,
    /// Recuadro de firma visible decidido para la petición (ADR-0019).
    pub visible: SiteVisibleSignature,
    /// Filtro de certificados solicitado por la sede.
    pub filter: SiteFilter,
    /// Si el documento contiene firmas que no se pueden interpretar.
    pub unregistered_signatures: bool,
    /// Pistas de guardado, si esta firma viene de `signandsave`.
    pub saving: Option<Box<SavingHints>>,
}

/// Datos del consentimiento del lote remoto, que se firma sin documento delante.
#[derive(Clone, Debug)]
pub struct BatchConsent {
    /// El lote tal y como lo pidió la sede.
    pub request: BatchRequest,
    /// Cuántas firmas lleva el lote.
    pub signs: usize,
    /// Certificados aceptados por la sede, ya cribados.
    pub certificates: Vec<ListedCertificate>,
    /// El asa del certificado recordado cuando `sticky` lo resolvió sin preguntar.
    pub already_chosen: Option<String>,
}

/// El resumen de un elemento del lote local: ni su ruta ni su contenido cruzan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBatchItem {
    /// El identificador con el que la sede nombra el elemento.
    pub id: String,
    /// El formato con el que se firma, con `auto` ya resuelto por la cabecera.
    pub format: Format,
    /// La operación que el lote pide sobre el elemento.
    pub round: SignatureRound,
}

/// Datos del consentimiento del lote local, que enseña qué es cada elemento antes de firmar.
#[derive(Clone, Debug)]
pub struct LocalBatchConsent {
    /// El lote tal y como lo pidió la sede.
    pub request: BatchRequest,
    /// Las firmas del lote, ya leídas.
    pub batch: LocalBatch,
    /// Un resumen por elemento, en el orden en que la sede los declaró.
    pub items: Vec<LocalBatchItem>,
    /// Certificados aceptados por la sede, ya cribados.
    pub certificates: Vec<ListedCertificate>,
    /// El asa del certificado recordado cuando `sticky` lo resolvió sin preguntar.
    pub already_chosen: Option<String>,
}

/// Pistas de guardado de `signandsave`, calculadas antes de firmar y usadas tras la postfirma.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavingHints {
    /// Nombre de fichero propuesto (`signandsave` no declara `title`; siempre hay uno).
    pub filename: String,
    /// Extensiones admitidas por el filtro del diálogo de guardado.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones, si la sede la declaró.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, nunca la fuente de la escritura.
    pub starting_folder: Option<String>,
}

impl SavingHints {
    /// El paso de guardado tras la postfirma, contestando con el mismo par que `sign`.
    pub fn into_consent(self, signed: &SiteSignature) -> SavingConsent {
        SavingConsent {
            data: signed.signature.clone(),
            title: None,
            filename: Some(self.filename),
            extensions: self.extensions,
            description: self.description,
            starting_folder: self.starting_folder,
            signer_der: Some(signed.signer_der.clone()),
        }
    }
}

/// Datos para el diálogo de guardado del portal: el nombre cruza, la ruta nunca (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavingConsent {
    /// El fichero que la sede pide guardar, en bytes.
    pub data: Vec<u8>,
    /// Título del diálogo declarado por la sede.
    pub title: Option<String>,
    /// Nombre de fichero propuesto por la sede.
    pub filename: Option<String>,
    /// Extensiones admitidas por el filtro del diálogo.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones declarada por la sede.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, si la declaró (`signandsave`; `save` no la tiene).
    pub starting_folder: Option<String>,
    /// El DER del firmante con el que contestar si esto viene de `signandsave`, `None` en `save`.
    pub signer_der: Option<Vec<u8>>,
}

/// Datos para el selector de carga del portal: el nombre cruza, la ruta nunca (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadingConsent {
    /// Título del selector declarado por la sede.
    pub title: Option<String>,
    /// Extensiones admitidas por el filtro del selector.
    pub extensions: Vec<String>,
    /// Descripción del filtro de extensiones declarada por la sede.
    pub description: Option<String>,
    /// Carpeta inicial sugerida por la sede, nunca la fuente de la lectura.
    pub starting_folder: Option<String>,
    /// Si la sede pide varios ficheros (`multiload=true`) o uno solo.
    pub multiple: bool,
    /// Si este selector viene de `signandsave` sin `dat`, la petición que continúa con el
    /// documento elegido; `None` cuando es un `load` corriente que contesta a la sede.
    pub to_sign: Option<Box<SignAndSaveRequest>>,
}

/// Desenlace del trámite para la sede y para la ventana.
#[derive(Debug)]
pub enum SiteOutcome {
    /// Certificado entregado por la persona, en DER.
    Certificate(Vec<u8>),
    /// Firma producida para la sede junto con el certificado firmante.
    Signature {
        /// El DER del firmante.
        signer_der: Vec<u8>,
        /// La firma, en el formato que pidió la sede.
        signature: Vec<u8>,
    },
    /// El fichero pedido por la sede queda escrito donde la persona eligió.
    Saved,
    /// Los ficheros que la persona eligió, con su nombre y su contenido.
    Loaded(Vec<(String, Vec<u8>)>),
    /// Trámite cancelado por la persona.
    Cancelled,
    /// Rechazo con su situación, que el adaptador traduce al cable y a la ventana.
    Refused(SiteRefusal),
    /// Rechazo directo del protocolo.
    RefusedByTheProtocol(Refusal),
    /// Resultado del lote remoto tal cual llegó del postsigner (JSON o XML), con el DER del
    /// firmante cuando la sede pidió `needcert`.
    Batch {
        /// El resultado del postsigner, sin tocar.
        result: Vec<u8>,
        /// El DER del firmante, presente solo cuando la sede pidió `needcert`.
        signer_der: Option<Vec<u8>>,
    },
}

impl SiteOutcome {
    /// La situación del rechazo, si lo hay.
    pub fn refusal(&self) -> Option<&SiteRefusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            _ => None,
        }
    }
}

/// Momento en el que se encuentra el trámite para la ventana de sede.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// Canal abierto esperando la petición de la sede.
    Waiting,
    /// Consentimiento de identificación con certificados cribados.
    AskingForConsent {
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
    },
    /// Consentimiento de firma de documento con certificados cribados.
    AskingToSign {
        /// Identificador del documento para la ventana.
        document: String,
        /// Formato de firma que la sede pidió.
        format: Format,
        /// Modalidad de firma solicitada.
        round: SignatureRound,
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
        /// Si el documento contiene firmas que no se pueden interpretar.
        unregistered_signatures: bool,
    },
    /// Consentimiento del lote remoto, sin documento y con cuántas firmas lleva.
    AskingToSignTheBatch {
        /// Cuántas firmas lleva el lote.
        signs: usize,
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
        /// El asa del certificado que `sticky` ya resolvió, si lo resolvió.
        already_chosen: Option<String>,
    },
    /// Consentimiento del lote local, con el resumen de cada uno de sus elementos.
    AskingToSignTheLocalBatch {
        /// Un resumen por elemento, en el orden en que la sede los declaró.
        items: Vec<LocalBatchItem>,
        /// Filas ya cribadas en orden de presentación.
        certificates: Vec<ListedCertificate>,
        /// El asa del certificado que `sticky` ya resolvió, si lo resolvió.
        already_chosen: Option<String>,
    },
    /// Trámite sin certificados disponibles.
    NoCertificate {
        /// Razón por la que no hay certificado.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona.
        owned: usize,
    },
    /// La sede pide guardar un fichero: el nombre que se muestra, nunca la ruta.
    Saving {
        /// Nombre de fichero propuesto por la sede, si lo hay.
        filename: Option<String>,
    },
    /// La sede pide cargar uno o varios ficheros.
    Loading {
        /// Si la sede pide varios ficheros (`multiload=true`) o uno solo.
        multiple: bool,
    },
    /// Canal con la sede no disponible.
    NoChannel(NoChannel),
    /// Rechazo del protocolo sin canal por el que responder.
    RefusedWithoutChannel(Refusal),
}

/// Motivo por el que no hay canal abierto con la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoChannel {
    /// No se pudo abrir el canal en los puertos asignados o con el material TLS.
    ChannelNotOpened,
    /// La CA local no está registrada en ningún almacén NSS (ADR-0005).
    LocalCaMissing,
}

/// El códec del protocolo: lee la petición y escribe el desenlace; habla este vocabulario y no el del dominio porque un rechazo lleva dentro lo que dijeron los vecinos.
pub trait ProtocolCodec {
    /// Lee la operación que llegó por el canal abierto.
    fn decode(&self, message: &AfirmaUrl) -> SiteRequest;

    /// Línea exacta que se escribe en el canal para el desenlace dado.
    fn encode(&self, outcome: &SiteOutcome) -> String;
}
