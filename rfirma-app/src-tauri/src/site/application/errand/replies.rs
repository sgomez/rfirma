//! Respuestas finales del trámite para la sede y la ventana (ADR-0009).

use crate::identity::adapters::pkcs11;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::store::Store;
use crate::site::domain::protocol::SiteFilter;

use super::outcome::{ErrandStep, NoCertificate, SiteOutcome};
use super::state::LiveErrand;
use crate::signing::application::filtering;
use crate::signing::ports::FilterEngine;
use crate::site::application::session::SiteRefusal;
use crate::site::application::session::SiteSignature;

/// Caso de uso: la persona consiente identificarse y entrega el certificado.
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
        Err(error) => return over(live, SiteOutcome::Refused(SiteRefusal::Token(error))),
    };

    identity_handed_over(engine, filter, &found, handle, listed, live)
}

/// Caso de uso: la persona entrega un certificado concreto tras comprobar el filtro.
pub fn identity_handed_over<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    found: &[crate::identity::adapters::pkcs11::TokenCertificate],
    handle: &str,
    listed: &ListedCertificates,
    live: &LiveErrand,
) -> SiteOutcome {
    let chosen =
        match filtering::usable_certificate_for_the_site(engine, filter, found, handle, listed) {
            Ok(chosen) => chosen,
            Err(error) => {
                return over(
                    live,
                    SiteOutcome::Refused(SiteRefusal::NotUsableForTheSite(error)),
                )
            }
        };

    over(live, SiteOutcome::Certificate(chosen.der().to_vec()))
}

/// Caso de uso: la persona entrega la firma completada.
pub fn signature_handed_over(live: &LiveErrand, signed: &SiteSignature) -> SiteOutcome {
    over(
        live,
        SiteOutcome::Signature {
            signer_der: signed.signer_der.clone(),
            signed: signed.signed.clone(),
        },
    )
}

/// Caso de uso: la firma falla y se notifica el rechazo correspondiente a la sede.
pub fn the_signature_did_not_come_out(live: &LiveErrand, refusal: SiteRefusal) -> SiteOutcome {
    over(live, SiteOutcome::Refused(refusal))
}

/// Caso de uso: la persona cancela el trámite.
pub fn declined(live: &LiveErrand) -> SiteOutcome {
    over(live, SiteOutcome::Cancelled)
}

/// Paso cuando la persona no tiene ningún certificado instalado.
pub(super) fn no_certificate_at_all() -> ErrandStep {
    ErrandStep::NoCertificate {
        reason: NoCertificate::NotOne,
        owned: 0,
        answered: None,
    }
}

/// Paso cuando la sede excluye todos los certificados instalados.
pub(super) fn no_certificate_the_site_accepts(live: &LiveErrand, owned: usize) -> ErrandStep {
    let answered = over(
        live,
        SiteOutcome::Refused(SiteRefusal::NoCertificateTheSiteAccepts),
    );
    ErrandStep::NoCertificate {
        reason: NoCertificate::TheSiteExcludedThemAll,
        owned,
        answered: Some(answered),
    }
}

/// Contesta a la sede y cierra el trámite.
pub(super) fn answering(live: &LiveErrand, reply: SiteOutcome) -> ErrandStep {
    ErrandStep::Answering(over(live, reply))
}

/// Envía el desenlace a la sede y finaliza el trámite activo.
pub(super) fn over(live: &LiveErrand, reply: SiteOutcome) -> SiteOutcome {
    live.answer_the_site(&reply);
    live.end();
    reply
}
