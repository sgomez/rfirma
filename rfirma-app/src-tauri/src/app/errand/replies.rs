//! **Las respuestas finales del trámite** (RD-09): lo que la sede recibe
//! cuando la persona ya ha decidido, y el único sitio que escribe en el cable
//! (ID-322).
//!
//! Todo lo que sale de aquí es un [`SiteOutcome`] sin versión: quien lo pone
//! en la forma del cable es el códec negociado, desde
//! [`LiveErrand::answer_the_site`]. Y todo sale **en el acto** (ID-275): la
//! sede no espera a que nadie cierre una ventana.

use crate::commands::Failure;
use crate::memory::ListedCertificates;
use crate::pkcs11::{self, Store};
use crate::protocol::{SafCode, SiteFilter, WireAnswer};

use super::outcome::{ErrandStep, NoCertificate, SiteOutcome};
use super::state::LiveErrand;
use crate::app::filtering::{self, FilterEngine};
use crate::app::frontier;
use crate::app::signing::{SiteRefusal, SiteSignature};

/// **Caso de uso.** La persona se identifica: la sede recibe el certificado en
/// el acto (ID-275, ID-276).
///
/// Y como [`attend_operation`], **éste lista el token**: la decisión entera es
/// de [`identity_handed_over`].
pub fn identify_with<E: FilterEngine>(
    engine: &E,
    stores: &[Store],
    filter: &SiteFilter,
    handle: &str,
    listed: &ListedCertificates,
    live: &LiveErrand,
) -> SiteOutcome {
    let found = match pkcs11::list_certificates_across(stores) {
        Ok(found) => found,
        Err(error) => {
            let code = frontier::code_of_token(error.situation());
            return over(
                live,
                SiteOutcome::Refused {
                    answer: WireAnswer::refused(code),
                    failure: error.into(),
                },
            );
        }
    };

    identity_handed_over(engine, filter, &found, handle, listed, live)
}

/// **Caso de uso.** Lo que la sede recibe cuando la persona se identifica con
/// uno de los certificados que tenía delante.
///
/// El filtro se vuelve a comprobar antes de entregar nada (ID-259): que el
/// certificado estuviera en la lista que la ventana enseñó no basta, porque la
/// ventana no es quien hace cumplir lo que pidió la sede.
pub fn identity_handed_over<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    found: &[crate::pkcs11::TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
    live: &LiveErrand,
) -> SiteOutcome {
    let chosen =
        match filtering::usable_certificate_for_the_site(engine, filter, found, handle, listed) {
            Ok(chosen) => chosen,
            // El certificado que la ventana señaló ya no está, ya no sirve o la
            // sede ya no lo acepta: para ella, ninguno que valga.
            Err(failure) => {
                return over(
                    live,
                    SiteOutcome::Refused {
                        answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
                        failure,
                    },
                )
            }
        };

    over(live, SiteOutcome::Certificate(chosen.der().to_vec()))
}

/// **Caso de uso.** Lo que la sede recibe cuando la firma ha terminado
/// (ID-275).
///
/// El certificado delante y la firma detrás, separados por `|`, los dos en
/// Base64 URL-safe: es lo que `processSignResponse` parte
/// (`autoscript.js:2512`-`2549`).
pub fn signature_handed_over(live: &LiveErrand, signed: &SiteSignature) -> SiteOutcome {
    over(
        live,
        SiteOutcome::Signature {
            signer_der: signed.signer_der.clone(),
            signed: signed.signed.clone(),
        },
    )
}

/// **Caso de uso.** La firma no ha salido, y la sede se entera en el acto
/// (ID-275).
///
/// **El código lo trae la situación, no este sitio** (ID-292). Después del
/// consentimiento se falla por muchas cosas que no son «la firma no ha
/// salido» —el token que no está o está bloqueado, el certificado que la sede
/// ya no acepta, la política que declaró y no se puede aplicar, el sello del
/// ADR-0016 que no cuadra—, y todas tienen código propio en el catálogo. Quien
/// lo decide es [`super::signing`], donde la situación todavía tiene tipo, y
/// llega aquí dentro de [`SiteRefusal`] ya resuelto: `SAF_09` es lo que sale
/// cuando de verdad es la firma la que no ha salido, y no el saco de todo.
///
/// La precisión no se pierde por el camino: el [`Failure`] entero se queda para
/// la ventana, que es la que puede contarle a la persona qué ha pasado (ID-29,
/// ID-291).
///
/// **El PIN equivocado no pasa por aquí**, y esa ausencia es la decisión: la
/// firma en el token no cierra el trámite porque la persona puede volver a
/// teclearlo, igual que en el recorrido local. Lo que cierra el trámite es la
/// prefirma que no abre el ciclo y la postfirma que no ensambla.
pub fn the_signature_did_not_come_out(live: &LiveErrand, refusal: SiteRefusal) -> SiteOutcome {
    over(
        live,
        SiteOutcome::Refused {
            answer: WireAnswer::refused(refusal.code()),
            failure: refusal.into_failure(),
        },
    )
}

/// **Caso de uso.** La persona ha dicho que no: `CANCEL` sale en el acto
/// (ID-275, ID-293).
pub fn declined(live: &LiveErrand) -> SiteOutcome {
    over(live, SiteOutcome::Cancelled)
}

/// **La persona no tiene ni un certificado** (ID-278, ID-341).
///
/// **No sale nada al cable y el trámite sigue vivo**, y es lo único que lo
/// separa de que la sede los excluya a todos: aquí hay arreglo y no depende de
/// la sede —instalar uno y volver a mirar—, así que contestarle ya sería cerrar
/// la única puerta que quedaba abierta. Las dos salidas de la ventana —el pie y
/// la cruz— siguen siendo [`declined`], y la sede recibe su `CANCEL` en cuanto
/// la persona diga que no (ID-340).
pub(super) fn no_certificate_at_all() -> ErrandStep {
    ErrandStep::NoCertificate {
        reason: NoCertificate::NotOne,
        owned: 0,
        answered: None,
    }
}

/// **La sede los ha excluido todos** (ID-278): recibe su `SAF_19` en el acto y
/// la ventana enseña cuántos tiene la persona.
///
/// Los dos a la vez, y por eso está en una sola función: el código sale al
/// cable —instalar otro certificado no arreglaría nada, así que no hay nada que
/// esperar— y el paso que se devuelve lleva la misma decisión contada para la
/// ventana.
pub(super) fn no_certificate_the_site_accepts(live: &LiveErrand, owned: usize) -> ErrandStep {
    let answered = over(
        live,
        SiteOutcome::Refused {
            answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
            failure: Failure::new(
                "certificateNotFound",
                "no queda ningun certificado que la sede acepte",
            ),
        },
    );
    ErrandStep::NoCertificate {
        reason: NoCertificate::TheSiteExcludedThemAll,
        owned,
        answered: Some(answered),
    }
}

/// Contesta y cierra el trámite: la sede ya tiene lo suyo.
pub(super) fn answering(live: &LiveErrand, reply: SiteOutcome) -> ErrandStep {
    ErrandStep::Answering(over(live, reply))
}

/// Lo mismo, cuando lo que se devuelve es la respuesta y nada más.
///
/// **Éste es el único sitio que escribe en el cable** (ID-322): la línea la
/// escribe el códec negociado a partir del desenlace —el certificado, la firma,
/// `CANCEL` o un `SAF_` del catálogo cerrado—, nunca un mensaje redactado
/// aquí. Y sale antes de cerrar el trámite, porque cerrarlo es lo que gasta el
/// asa.
pub(super) fn over(live: &LiveErrand, reply: SiteOutcome) -> SiteOutcome {
    live.answer_the_site(&reply);
    live.end();
    reply
}
