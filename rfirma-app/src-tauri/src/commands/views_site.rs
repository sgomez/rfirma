//! **Lo que cruza a la ventana de sede** (ID-80, ID-338, ID-341): los tipos de
//! salida del trámite y las conversiones que los producen.
//!
//! Aparte de [`super::views`] por lo mismo que [`super::rubric`]: son los
//! mismos dos papeles —tipos que cruzan y conversiones— pero de otra ventana,
//! y quien trabaja en la de sede lee un fichero y no el de la principal.
//! Ninguno lleva una ruta del anfitrión (ADR-0011): el documento que manda la
//! sede se nombra con un asa acuñada, y su fichero de paso se borra al contestar
//! (ID-286).
//!
//! La entrada es un [`Moment`] del trámite, y la conversión es
//! [`SiteErrandView::from`]: el trámite decide en qué momento está, y aquí sólo
//! se pone en la forma que cruza el IPC.

use serde::Serialize;

use crate::app::errand::{Moment, NoCertificate, NoChannel};
use crate::protocol::{Refusal, RefusalSituation, SignatureRound};

use super::views::CertificateView;

/// **El trámite de sede, tal como lo recibe su ventana** (ID-338, ID-339).
///
/// Viaja por un **evento** y no por una orden: el trámite empuja cada momento
/// nuevo, y que no llegue nunca es la respuesta normal, porque la mayoría de
/// los arranques no vienen de una sede.
///
/// Al abrirse la ventana sólo se sabe que el canal está en pie: el origen y la
/// operación llegan con la petición, que es lo que la sede manda **después**
/// por el canal ya abierto. Por eso aquí no hay ni ruta ni identificador de
/// documento: detrás de una espera no hay ningún documento todavía.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteErrandView {
    /// Quién pide la firma, nombrado **a secas** (ID-271). Durante la espera
    /// todavía no se sabe.
    pub origin: Option<String>,
    /// En qué momento de la secuencia está la ventana.
    pub stage: SiteStageView,
}

impl SiteErrandView {
    /// El trámite recién abierto: el canal está en pie y la petición no ha
    /// llegado.
    pub fn waiting() -> Self {
        Self {
            origin: None,
            stage: SiteStageView::Waiting,
        }
    }

    /// **El canal no se ha abierto y ya no va a abrirse** (ID-341): o todos los
    /// puertos que sorteó la sede estaban ocupados, o la CA local no está en
    /// ningún almacén NSS y ningún navegador va a intentarlo siquiera.
    ///
    /// No hay socket por el que decirlo, así que se dice aquí: una ventana que
    /// aparece y desaparece en silencio es indistinguible de un rFirma roto.
    pub fn no_channel(reason: NoChannelView) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::NoChannel { reason },
        }
    }

    /// **El rechazo que no tiene socket por el que salir** (ID-341): sin
    /// `ports` en la URL, o con todos ocupados.
    ///
    /// Lo que cruza es la **situación** clasificada y el detalle crudo, nunca
    /// una frase redactada aquí (ADR-0009, ID-29, ID-291): la prosa la pone la
    /// ventana y el detalle es lo único accionable de esa pantalla, para
    /// llevárselo a quien mantiene la sede.
    pub fn refused(refusal: &Refusal) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::Outcome {
                outcome: SiteOutcomeView::Refused {
                    situation: refusal.situation().into(),
                    detail: refusal.detail().to_owned(),
                },
            },
        }
    }

    /// **No hay ningún certificado con el que seguir** (ID-278), con su motivo
    /// y cuántos tiene la persona en su almacén.
    pub fn without_certificates(reason: NoCertificateView, owned: usize) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::NoCertificate { reason, owned },
        }
    }

    /// **El momento del consentimiento** (ID-272, ID-276): la sede pidió
    /// identificación y éstas son las filas que acepta.
    ///
    /// Enseñarlas es lo único que pasa: la sede no recibe nada hasta que la
    /// persona conteste (ID-275).
    pub fn asking_for_consent(certificates: Vec<CertificateView>) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::AskingForConsent { certificates },
        }
    }

    /// **El momento del consentimiento de una firma** (ID-272): la sede manda
    /// un documento, dice si lo que pide es firmarlo o cofirmarlo, y éstas son
    /// las filas que acepta.
    ///
    /// El documento cruza por su **asa**, que es como lo nombra la ventana para
    /// leerlo con [`super::read_document`]: la ruta del fichero de paso no sale
    /// de aquí, y de ella no queda rastro en cuanto el trámite conteste
    /// (ID-286, ADR-0011).
    pub fn asking_to_sign(
        document: &str,
        round: SignatureRound,
        certificates: &[CertificateView],
        unregistered_signatures: bool,
    ) -> Self {
        Self {
            origin: None,
            stage: SiteStageView::AskingToSign {
                document: document.to_owned(),
                round: round.into(),
                certificates: certificates.to_vec(),
                unregistered_signatures,
            },
        }
    }
}

impl From<&Moment> for SiteErrandView {
    /// **El momento del trámite, puesto en la forma que cruza el IPC.** Es
    /// traducir y nada más: quién decide el momento es el trámite
    /// ([`crate::app::errand`]) y el arranque ([`crate::app::startup`]).
    fn from(moment: &Moment) -> Self {
        match moment {
            Moment::Waiting => Self::waiting(),
            Moment::AskingForConsent { certificates } => {
                Self::asking_for_consent(certificates.clone())
            }
            Moment::AskingToSign {
                document,
                round,
                certificates,
                unregistered_signatures,
            } => Self::asking_to_sign(document, *round, certificates, *unregistered_signatures),
            Moment::NoCertificate { reason, owned } => {
                Self::without_certificates((*reason).into(), *owned)
            }
            Moment::NoChannel(NoChannel::ChannelNotOpened) => {
                Self::no_channel(NoChannelView::ChannelNotOpened)
            }
            Moment::NoChannel(NoChannel::LocalCaMissing) => {
                Self::no_channel(NoChannelView::LocalCaMissing)
            }
            Moment::RefusedWithoutChannel(refusal) => Self::refused(refusal),
        }
    }
}

/// Cuál de las dos firmas ha pedido la sede, tal y como se le cuenta a la
/// persona.
///
/// Se nombran como los verbos con los que la sede las pide —`sign` y
/// `cosign`— y no como las variantes de
/// [`SignatureRound`](crate::protocol::SignatureRound), porque lo que la
/// ventana enseña es lo que la sede pidió.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SignatureRoundView {
    /// La sede manda un documento y pide su firma.
    Sign,
    /// La sede manda un documento ya firmado y pide una firma más encima.
    Cosign,
}

impl From<SignatureRound> for SignatureRoundView {
    fn from(round: SignatureRound) -> Self {
        match round {
            SignatureRound::First => Self::Sign,
            SignatureRound::Again => Self::Cosign,
        }
    }
}

/// El momento de la secuencia que la ventana de sede enseña.
///
/// Tres momentos: la espera —lo único que hay **antes** de que la sede mande su
/// petición por el canal— y los dos consentimientos, el de una identificación y
/// el de una firma.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteStageView {
    /// El canal está abierto y la petición no ha llegado. Cuánto se espera
    /// antes de decir algo lo decide el reloj de la ventana, no el backend.
    Waiting,
    /// La sede pide identificación, y éstos son los certificados que acepta
    /// (ID-276). La persona elige uno o dice que no; hasta entonces la sede no
    /// recibe nada (ID-272, ID-275).
    AskingForConsent {
        /// Las filas ya cribadas, en el orden en el que se enseñan.
        certificates: Vec<CertificateView>,
    },
    /// La sede pide una firma o una cofirma sobre el documento que manda, y
    /// éstos son los certificados que acepta (ID-272). La persona consiente y
    /// teclea el PIN, o dice que no; hasta entonces la sede no recibe nada
    /// (ID-275).
    #[serde(rename_all = "camelCase")]
    AskingToSign {
        /// El asa con la que la ventana nombra el documento que manda la sede,
        /// la misma que lee [`super::read_document`]. **No es una ruta**: del
        /// documento de una sede no queda rastro (ID-286, ADR-0011).
        document: String,
        /// Si lo que se pide es firmar o cofirmar, que es parte de lo que hay
        /// que contarle a la persona antes de que consienta.
        round: SignatureRoundView,
        /// Las filas ya cribadas, en el orden en el que se enseñan.
        certificates: Vec<CertificateView>,
        /// Que el documento trae **firmas que rFirma no sabe leer** (ID-297).
        ///
        /// Viaja **dentro** del consentimiento y no como un rechazo aparte,
        /// para que la pregunta quepa dentro de él: decir que no a esto es
        /// decir que no al trámite, y sale `CANCEL` (ID-299, ID-303).
        unregistered_signatures: bool,
    },
    /// **El canal no se ha abierto y ya no va a abrirse** (ID-341). No hay
    /// socket por el que hablar, así que el desenlace es de la ventana y de
    /// nadie más.
    NoChannel {
        /// Por qué no lo hay. Es lo único que se dice: la reparación —las dos
        /// recetas de navegador y la dirección del ajuste de red local de
        /// Chrome, que se copia y no se pulsa— la pinta la ventana, que no
        /// diagnostica.
        reason: NoChannelView,
    },
    /// El trámite acabó y esto es lo que la ventana enseña. La sede, cuando
    /// tenía canal, ya recibió su código por él (ID-248).
    Outcome {
        /// Cómo acabó.
        outcome: SiteOutcomeView,
    },
    /// **No hay ningún certificado con el que seguir** (ID-278): no es una
    /// variante del consentimiento, porque aquí no hay nada que consentir ni
    /// nada que elegir.
    #[serde(rename_all = "camelCase")]
    NoCertificate {
        /// Cuál de las dos situaciones es, que es lo que decide si hay arreglo.
        reason: NoCertificateView,
        /// Cuántos certificados tiene la persona. Es estado de **su** almacén,
        /// y nunca cuáles descartó la sede ni con qué criterio (ID-277).
        owned: usize,
    },
}

/// Por qué no hay canal por el que hablar con la sede (ID-341).
///
/// Las dos son **conclusiones medidas**, no sospechas: la primera la da el
/// transporte al no devolver ningún canal, y la segunda sale de haber abierto
/// los perfiles NSS y no haber dejado la CA local en ninguno
/// (`TrustOutcome::nowhere`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoChannelView {
    /// El canal no se ha podido abrir: sin puertos libres entre los que
    /// sorteó la sede (ID-215), sin material TLS utilizable, o sin llegar a
    /// escuchar. Las tres se reparan igual, así que cruzan como una sola.
    ChannelNotOpened,
    /// La CA local no ha quedado en ningún almacén NSS: sin ella ningún
    /// navegador llega siquiera a intentar el canal (ID-329).
    LocalCaMissing,
}

/// Cómo acabó el trámite, tal como lo enseña la ventana.
///
/// Sólo el rechazo por ahora: firmado y cancelado los enseña el recorrido que
/// los produce, y aquí sólo entra lo que se decide **antes** de que haya nada
/// que consentir.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SiteOutcomeView {
    /// rFirma rechazó la petición.
    Refused {
        /// La situación **clasificada** (ADR-0009, ID-29): la frase la escribe
        /// la ventana en el idioma de la persona.
        situation: RefusalSituationView,
        /// El detalle crudo, sin traducir ni recortar. Es lo único accionable
        /// de la pantalla, y **no sale al cable** (ID-291).
        detail: String,
    },
}

/// Las situaciones de rechazo que la ventana sabe nombrar (ID-341).
///
/// Es [`crate::protocol::RefusalSituation`] tal y como cruza: un nombre y nada
/// más, porque el texto es del catálogo de la ventana (ADR-0009, ID-291).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefusalSituationView {
    /// `signaturePages=append` (ID-284).
    AppendedSignaturePage,
    /// Un criterio de filtro fuera de la lista blanca (ID-256).
    UnsupportedFilter,
    /// Una versión del protocolo que aquí no se habla (ID-251).
    UnsupportedProtocolVersion,
    /// La petición de firma no trae `format`.
    MissingFormat,
    /// Ya hay un trámite de sede vivo (ID-280).
    ErrandInFlight,
    /// Cualquier otro: la ventana lo cuenta en general y enseña el detalle.
    Unknown,
}

impl From<RefusalSituation> for RefusalSituationView {
    fn from(situation: RefusalSituation) -> Self {
        match situation {
            RefusalSituation::AppendedSignaturePage => Self::AppendedSignaturePage,
            RefusalSituation::UnsupportedFilter => Self::UnsupportedFilter,
            RefusalSituation::UnsupportedProtocolVersion => Self::UnsupportedProtocolVersion,
            RefusalSituation::MissingFormat => Self::MissingFormat,
            RefusalSituation::ErrandInFlight => Self::ErrandInFlight,
            RefusalSituation::Unknown => Self::Unknown,
        }
    }
}

impl From<NoCertificate> for NoCertificateView {
    fn from(reason: NoCertificate) -> Self {
        match reason {
            NoCertificate::NotOne => Self::None,
            NoCertificate::TheSiteExcludedThemAll => Self::Excluded,
        }
    }
}

/// Por qué no queda ningún certificado con el que seguir (ID-278).
///
/// Las dos **se sienten distintas porque la salida es distinta**: una tiene
/// arreglo y no depende de la sede; la otra no lo tiene, porque quien decide es
/// la sede.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NoCertificateView {
    /// La persona no tiene ninguno instalado. Instalar uno lo arregla, y por
    /// eso el trámite **sigue vivo** mientras esta pantalla está delante.
    None,
    /// La sede los ha excluido todos. Instalar otro no arregla nada, y la sede
    /// ya recibió su `SAF_19` (ID-275).
    Excluded,
}

#[cfg(test)]
mod tests {
    use super::{Moment, SignatureRound, SignatureRoundView};
    use super::{NoCertificateView, NoChannelView, RefusalSituation, SiteErrandView};

    /// **Los tres callejones sin salida, tal y como cruzan** (ID-341).
    ///
    /// Los nombres están fijados aquí porque son el contrato con la ventana:
    /// lo que cruza es una situación clasificada y su detalle crudo, y ni una
    /// frase redactada en el backend (ADR-0009, ID-29, ID-291).
    #[test]
    fn the_dead_ends_cross_named_and_never_written_out() {
        assert_eq!(
            serde_json::to_value(SiteErrandView::no_channel(NoChannelView::ChannelNotOpened))
                .expect("el callejon cruza"),
            serde_json::json!({
                "origin": null,
                "stage": { "kind": "noChannel", "reason": "channelNotOpened" },
            })
        );
        assert_eq!(
            serde_json::to_value(SiteErrandView::no_channel(NoChannelView::LocalCaMissing))
                .expect("el callejon cruza"),
            serde_json::json!({
                "origin": null,
                "stage": { "kind": "noChannel", "reason": "localCaMissing" },
            })
        );
        assert_eq!(
            serde_json::to_value(SiteErrandView::without_certificates(
                NoCertificateView::None,
                0
            ))
            .expect("el callejon cruza"),
            serde_json::json!({
                "origin": null,
                "stage": { "kind": "noCertificate", "reason": "none", "owned": 0 },
            })
        );
    }

    /// Y el rechazo sin canal cruza con **la situación y el detalle**: la
    /// situación es la que la ventana sabe nombrar, y el detalle lo único
    /// accionable de esa pantalla.
    #[test]
    fn a_refusal_without_a_channel_crosses_with_its_situation_and_its_detail() {
        let refusal = crate::protocol::Refusal::new(
            crate::protocol::SafCode::UnsupportedProcedure,
            "la sede declara la version de protocolo 3",
        )
        .because(RefusalSituation::UnsupportedProtocolVersion);

        assert_eq!(
            serde_json::to_value(SiteErrandView::refused(&refusal)).expect("el rechazo cruza"),
            serde_json::json!({
                "origin": null,
                "stage": {
                    "kind": "outcome",
                    "outcome": {
                        "kind": "refused",
                        "situation": "unsupportedProtocolVersion",
                        "detail": "la sede declara la version de protocolo 3",
                    },
                },
            })
        );
    }

    /// La ronda cruza con el nombre del verbo que la sede usó, y no con el de
    /// la variante del protocolo: es lo que la ventana enseña.
    #[test]
    fn the_round_crosses_named_as_the_site_asked_for_it() {
        let view = SiteErrandView::from(&Moment::AskingToSign {
            document: "doc-1".to_owned(),
            round: SignatureRound::Again,
            certificates: Vec::new(),
            unregistered_signatures: false,
        });

        assert_eq!(
            serde_json::to_value(&view).expect("serializa")["stage"]["round"],
            serde_json::to_value(SignatureRoundView::Cosign).expect("serializa")
        );
    }
}
