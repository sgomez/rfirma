//! Respuestas finales del trámite para la sede y la ventana (ADR-0009).

use std::path::Path;

use crate::identity::domain::certificate::TokenCertificate;
use crate::site::domain::protocol::SiteFilter;
use crate::site::domain::signing::SiteSignature;

use super::outcome::{ErrandStep, NoCertificate, SiteOutcome};
use super::state::LiveErrand;
use crate::site::application::filtering;
use crate::site::application::session::SiteRefusal;
use crate::site::ports::{Certificates, FilterEngine, Scratch};

/// Caso de uso: la persona consiente identificarse y entrega el certificado.
pub fn identify_with<E: FilterEngine>(
    engine: &E,
    certificates: &dyn Certificates,
    filter: &SiteFilter,
    sticky: bool,
    handle: &str,
    live: &LiveErrand,
) -> SiteOutcome {
    let found = match certificates.listed() {
        Ok(found) => found,
        Err(error) => return over(live, SiteOutcome::Refused(SiteRefusal::Token(error))),
    };

    identity_handed_over(engine, filter, sticky, &found, handle, certificates, live)
}

/// Caso de uso: la persona entrega un certificado concreto tras comprobar el filtro.
pub fn identity_handed_over<E: FilterEngine>(
    engine: &E,
    filter: &SiteFilter,
    sticky: bool,
    found: &[TokenCertificate],
    handle: &str,
    certificates: &dyn Certificates,
    live: &LiveErrand,
) -> SiteOutcome {
    let chosen = match filtering::usable_certificate_for_the_site(
        engine,
        filter,
        found,
        handle,
        certificates,
    ) {
        Ok(chosen) => chosen,
        Err(error) => {
            return over(
                live,
                SiteOutcome::Refused(SiteRefusal::NotUsableForTheSite(error)),
            )
        }
    };

    if sticky {
        certificates.remember(chosen.reference());
    }

    over(live, SiteOutcome::Certificate(chosen.der().to_vec()))
}

/// Caso de uso: la persona entrega la firma completada.
pub fn signature_handed_over(live: &LiveErrand, signed: &SiteSignature) -> SiteOutcome {
    over(
        live,
        SiteOutcome::Signature {
            signer_der: signed.signer_der.clone(),
            signature: signed.signature.clone(),
        },
    )
}

/// Caso de uso: la persona entrega el resultado del lote remoto tal cual llegó del postsigner.
pub fn batch_handed_over(
    live: &LiveErrand,
    result: Vec<u8>,
    signer_der: Option<Vec<u8>>,
) -> SiteOutcome {
    over(live, SiteOutcome::Batch { result, signer_der })
}

/// Caso de uso: la firma falla y se notifica el rechazo correspondiente a la sede.
pub fn the_signature_did_not_come_out(live: &LiveErrand, refusal: SiteRefusal) -> SiteOutcome {
    over(live, SiteOutcome::Refused(refusal))
}

/// Caso de uso: la persona cancela el trámite.
pub fn declined(live: &LiveErrand) -> SiteOutcome {
    over(live, SiteOutcome::Cancelled)
}

/// Caso de uso: se escribe en la ruta que la persona eligió el fichero que pidió la sede.
pub fn saved(
    scratch: &dyn Scratch,
    path: &Path,
    data: &[u8],
    signer_der: Option<&[u8]>,
    live: &LiveErrand,
) -> SiteOutcome {
    match scratch.write(path, data) {
        Ok(()) => over(
            live,
            match signer_der {
                Some(signer_der) => SiteOutcome::Signature {
                    signer_der: signer_der.to_vec(),
                    signature: data.to_vec(),
                },
                None => SiteOutcome::Saved,
            },
        ),
        Err(detail) => over(
            live,
            SiteOutcome::Refused(SiteRefusal::CannotSaveData(detail)),
        ),
    }
}

/// Caso de uso: se leen los ficheros que la persona eligió y se entregan a la sede.
pub fn loaded(
    scratch: &dyn Scratch,
    chosen: &[(String, std::path::PathBuf)],
    live: &LiveErrand,
) -> SiteOutcome {
    let mut files = Vec::with_capacity(chosen.len());
    for (name, path) in chosen {
        match scratch.read(path) {
            Ok(bytes) => files.push((name.clone(), bytes)),
            Err(detail) => {
                return over(
                    live,
                    SiteOutcome::Refused(SiteRefusal::CannotLoadData(detail)),
                )
            }
        }
    }
    over(live, SiteOutcome::Loaded(files))
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
