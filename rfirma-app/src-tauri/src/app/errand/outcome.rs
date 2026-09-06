//! **El vocabulario de salida del trámite** (RD-02, RD-09): lo que el trámite
//! produce para la sede y lo que deja delante de la persona.
//!
//! Dos familias, y la diferencia es quién las recibe:
//!
//! - [`SiteOutcome`] es **lo que la sede recibe**, sin versión: el certificado
//!   elegido, la firma con su certificado, la cancelación o un rechazo con su
//!   situación. Cómo se escribe en el cable es cosa del códec
//!   ([`super::ports::ProtocolCodec::encode`]), no de aquí.
//! - [`ErrandStep`], [`SigningConsent`], [`NoCertificate`] y [`Moment`] son
//!   **lo que queda para la ventana**: el momento del consentimiento, el
//!   callejón sin certificado y, guardado para quien todavía no escuchaba, el
//!   último momento del trámite (ID-338).

use std::collections::BTreeMap;

use crate::commands::views::CertificateView;
use crate::commands::Failure;
use crate::protocol::{Refusal, SignatureRound, SiteFilter, SiteVisibleSignature, WireAnswer};

/// En qué queda la operación que llegó por el canal.
#[derive(Debug)]
pub enum ErrandStep {
    /// **El momento del consentimiento** (ID-272, ID-276): la ventana enseña
    /// estas filas y la persona decide. La sede no recibe nada todavía.
    AskingForConsent {
        /// Los certificados que la sede acepta, ya cribados.
        certificates: Vec<CertificateView>,
        /// Lo que la sede pide del listado, para volver a comprobarlo (ID-259).
        ///
        /// Viaja con las filas por lo mismo que dentro de [`SigningConsent`]:
        /// el filtro se vuelve a aplicar antes de entregar el certificado, y la
        /// ventana no puede devolver algo que nunca cruzo (ADR-0011).
        filter: SiteFilter,
    },
    /// **El momento del consentimiento de una firma** (ID-272): la ventana
    /// enseña el documento que la sede manda y estas filas, y la persona
    /// decide. La sede no recibe nada todavía.
    AskingToSign(SigningConsent),
    /// **No hay ningún certificado con el que seguir** (ID-278, ID-341): no es
    /// una variante del consentimiento, porque aquí no hay nada que consentir
    /// ni nada que elegir.
    NoCertificate {
        /// Cuál de las dos situaciones es. Lo que las separa es la salida: una
        /// tiene arreglo y la otra no.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona en su almacén. Es **su**
        /// estado, y nunca cuáles descartó la sede (ID-277).
        owned: usize,
        /// Lo que la sede ya ha recibido, cuando le tocaba recibir algo.
        ///
        /// `None` es [`NoCertificate::NotOne`]: no ha salido nada al cable y el
        /// trámite sigue vivo (ver [`super::replies::no_certificate_at_all`]).
        answered: Option<SiteOutcome>,
    },
    /// No hay nada que consentir: esto es lo que la sede recibe, y sale ya
    /// (ID-275).
    Answering(SiteOutcome),
}

impl ErrandStep {
    /// **El momento que este paso deja delante de la persona**, si deja
    /// alguno: los dos consentimientos y el callejón sin certificado. Lo que
    /// ya está contestado no es un momento nuevo del trámite —lo que la
    /// ventana enseñe de un desenlace es del #394—.
    pub fn moment(&self) -> Option<Moment> {
        match self {
            Self::AskingForConsent { certificates, .. } => Some(Moment::AskingForConsent {
                certificates: certificates.clone(),
            }),
            Self::AskingToSign(consent) => Some(Moment::AskingToSign {
                document: consent.document.clone(),
                round: consent.round,
                certificates: consent.certificates.clone(),
                unregistered_signatures: consent.unregistered_signatures,
            }),
            Self::NoCertificate { reason, owned, .. } => Some(Moment::NoCertificate {
                reason: *reason,
                owned: *owned,
            }),
            Self::Answering(_) => None,
        }
    }
}

/// Por qué no queda ningún certificado con el que seguir (ID-278).
///
/// Las dos **se tienen que sentir distintas porque la salida es distinta**: una
/// tiene arreglo y no depende de la sede, la otra no lo tiene porque quien
/// decide es la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoCertificate {
    /// La persona no tiene ni uno instalado. Instalar uno lo arregla, así que
    /// el trámite espera y la sede no recibe nada todavía.
    NotOne,
    /// La sede los ha excluido todos. Instalar otro no arregla nada, y ella ya
    /// tiene su `SAF_19` (ID-275).
    TheSiteExcludedThemAll,
}

/// Lo que hay delante de la persona cuando una sede pide una firma.
///
/// No es una vista: es lo que hace falta para **seguir** el trámite si se
/// consiente, y por eso lleva dentro cosas que la ventana no pinta —el filtro y
/// los `extraParams`—. Las dos se vuelven a usar en la prefirma: el filtro
/// porque se comprueba otra vez antes del PIN (ID-259) y los `extraParams`
/// porque son la política que la sede declaró (ID-266).
#[derive(Debug)]
pub struct SigningConsent {
    /// El identificador con el que la ventana nombra el documento de la sede.
    /// **No es una ruta** (ADR-0011).
    pub document: String,
    /// `sign` o `cosign`, que es lo que hay que contarle a la persona.
    pub round: SignatureRound,
    /// Los certificados que la sede acepta, ya cribados.
    pub certificates: Vec<CertificateView>,
    /// Los `extraParams` de la sede, **ya expandidos** (ID-266).
    pub from_the_site: BTreeMap<String, String>,
    /// Qué recuadro pide la sede, ya decidido (ID-282).
    ///
    /// **La prefirma no lo lee, y no es un olvido**: el recuadro lo coloca la
    /// sede en sus propios `extraParams` y ésos cruzan crudos al puente, así
    /// que no hay nada que emitir desde aquí. El trabajo de este campo ya está
    /// hecho cuando llega: lo hizo [`crate::protocol::visible_signature_of`]
    /// **antes** del consentimiento, al rechazar con `SAF_43` el recuadro que
    /// la sede exige y no puede colocar, y al decidir que un `optional` sin
    /// sitio se firma invisible. Lo que queda es el registro de esa decisión,
    /// que es lo que las pruebas de esta unidad miran.
    pub visible: SiteVisibleSignature,
    /// Lo que la sede pide del listado, para volver a comprobarlo (ID-259).
    pub filter: SiteFilter,
    /// Que el documento trae **firmas que rFirma no sabe leer** (ID-297).
    ///
    /// La pregunta vive **dentro** de este consentimiento y no en un sexto
    /// momento (ID-298): la ventana lo enseña con lo demás, y si la persona
    /// dice que no, lo que sale es `CANCEL` (ID-303). No hay recuento ni
    /// titulares detrás, y las firmas previas no se validan (ID-305).
    pub unregistered_signatures: bool,
}

/// **Lo que la sede recibe**, sin versión (RD-02), y lo que queda para la
/// ventana.
///
/// Los dos juntos en un tipo porque son **la misma decisión contada dos
/// veces**: el cable se lleva el código del catálogo cerrado y la ventana, la
/// situación entera con su detalle, que es lo que el ID-291 no deja salir.
///
/// Aquí no hay Base64 ni separadores: los bytes viajan como bytes y el código
/// como código, y cómo se escribe cada cosa en el cable lo decide el códec
/// ([`super::ports::ProtocolCodec::encode`], RD-03).
#[derive(Debug)]
pub enum SiteOutcome {
    /// El certificado que la persona entregó, en DER (ID-276).
    Certificate(Vec<u8>),
    /// La firma que la sede pidió, con el certificado que la produjo delante
    /// (ID-275).
    Signature {
        /// El DER del firmante.
        signer_der: Vec<u8>,
        /// El PDF firmado.
        signed: Vec<u8>,
    },
    /// La persona ha dicho que no (ID-293).
    Cancelled,
    /// La sede recibe el código; la ventana, la situación entera.
    Refused {
        /// Lo que sale al cable.
        answer: WireAnswer,
        /// Lo que se queda dentro y enseña la ventana (ID-29, ID-291).
        failure: Failure,
    },
    /// El rechazo es **del protocolo**, y esos nacen ya con su código: no hay
    /// situación nuestra detrás que traducir ni que enseñar traducida
    /// ([`crate::app::frontier`], ID-288). El detalle crudo viaja dentro y
    /// **no sale al cable** (ID-291).
    RefusedByTheProtocol(Refusal),
}

impl SiteOutcome {
    /// Lo que la ventana tiene que enseñar, cuando hay algo que enseñar.
    pub fn failure(&self) -> Option<&Failure> {
        match self {
            Self::Refused { failure, .. } => Some(failure),
            _ => None,
        }
    }
}

/// **En qué momento está el trámite**, tal y como lo guarda el estado para
/// la ventana de sede (ID-338, ID-341).
///
/// Es la memoria del trámite y no una vista: lo que cruza a la ventana es su
/// traducción, en `commands/`. Los seis momentos son los que la ventana sabe
/// enseñar: la espera, los dos consentimientos, el callejón sin certificado y
/// los dos callejones sin canal —el que no se abrió y el rechazo que no tuvo
/// por dónde salir—.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Moment {
    /// El canal está en pie y la petición de la sede no ha llegado.
    Waiting,
    /// La sede pidió identificación y éstas son las filas que acepta
    /// (ID-276).
    AskingForConsent {
        /// Las filas ya cribadas, en el orden en el que se enseñan.
        certificates: Vec<CertificateView>,
    },
    /// La sede manda un documento y pide su firma o su cofirma (ID-272).
    AskingToSign {
        /// El asa con la que la ventana nombra el documento de la sede.
        document: String,
        /// Si lo que se pide es firmar o cofirmar.
        round: SignatureRound,
        /// Las filas ya cribadas, en el orden en el que se enseñan.
        certificates: Vec<CertificateView>,
        /// Que el documento trae firmas que rFirma no sabe leer (ID-297).
        unregistered_signatures: bool,
    },
    /// **No hay ningún certificado con el que seguir** (ID-278).
    NoCertificate {
        /// Cuál de las dos situaciones es.
        reason: NoCertificate,
        /// Cuántos certificados tiene la persona.
        owned: usize,
    },
    /// **El canal no se ha abierto y ya no va a abrirse** (ID-341).
    NoChannel(NoChannel),
    /// **El rechazo que no tiene socket por el que salir** (ID-341): sin
    /// `ports` en la URL, o con todos ocupados.
    RefusedWithoutChannel(Refusal),
}

/// Por qué no hay canal por el que hablar con la sede (ID-341).
///
/// Las dos son **conclusiones medidas**, no sospechas: la primera la da el
/// transporte al no devolver ningún canal, y la segunda sale de haber abierto
/// los perfiles NSS y no haber dejado la CA local en ninguno.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoChannel {
    /// El canal no se ha podido abrir: sin puertos libres entre los que sorteó
    /// la sede (ID-215), sin material TLS utilizable, o sin llegar a escuchar.
    /// Las tres se reparan igual, así que se cuentan como una sola.
    ChannelNotOpened,
    /// La CA local no ha quedado en ningún almacén NSS: sin ella ningún
    /// navegador llega siquiera a intentar el canal (ID-329).
    LocalCaMissing,
}
