//! La firma que la sede manda hacer a su servidor trifásico, del certificado elegido a la firma que se entrega; no abre el ciclo del puente.

use crate::site::application::triphase::{signed_through_the_server, ServerAsk, ServerRun};
use crate::site::domain::signing::SiteSignature;
use crate::site::domain::triphase_server::server_url_of;
use crate::site::ports::{FilterEngine, PolicyEngine};

use super::state::{PendingSignature, ServerSignature};
use super::{
    the_chosen_and_its_secret, told_to_the_site, ConsentError, Consented, ErrandDesk, LiveErrand,
    Neighbours, SiteRefusal,
};

/// Apunta el certificado elegido y abre su secreto; sin `serverUrl` la sede recibe el rechazo ya.
pub(super) fn consented<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    pending: PendingSignature,
    server: ServerSignature,
    certificate: &str,
    live: &LiveErrand,
) -> Result<Consented, ConsentError> {
    let (chosen, secret) = the_chosen_and_its_secret(
        desk,
        &pending.filter,
        false,
        certificate,
        live,
        SiteRefusal::Signing,
    )?;
    if let Err(error) = server_url_of(&pending.from_the_site) {
        return Err(ConsentError::Refused(told_to_the_site(
            live,
            SiteRefusal::Triphase(error),
        )));
    }

    live.remember_signature(PendingSignature {
        through_the_server: Some(ServerSignature {
            chosen: Some(chosen),
            ..server
        }),
        ..pending
    });
    Ok(Consented::SigningWith(secret))
}

/// Hace las tres fases con el secreto ya tecleado y deja la firma lista para entregarla.
pub fn finish_the_server_signature<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    secret: &str,
    live: &LiveErrand,
) -> Result<(), ConsentError> {
    let pending = live
        .the_signature_consented()
        .ok_or(ConsentError::NothingPending)?;
    let server = pending
        .through_the_server
        .clone()
        .ok_or(ConsentError::NothingPending)?;
    let chosen = server.chosen.clone().ok_or(ConsentError::NothingPending)?;

    let signature = signed_through_the_server(
        &ServerRun {
            server: desk.triphase.as_ref(),
            token: &desk.neighbours,
            certificate: &chosen,
            secret,
        },
        &ServerAsk {
            format: server.format,
            round: server.round,
            algorithm: pending.algorithm,
            document: &server.document,
            from_the_site: &pending.from_the_site,
        },
    )
    .map_err(|refusal| ConsentError::Refused(told_to_the_site(live, refusal)))?;

    live.remember_signature(PendingSignature {
        through_the_server: Some(ServerSignature {
            signed: Some(SiteSignature {
                signature,
                signer_der: chosen.der().to_vec(),
            }),
            ..server
        }),
        ..pending
    });
    Ok(())
}

/// La firma que ya devolvió el servidor trifásico, si la hay.
pub(super) fn the_signature_from_the_server(live: &LiveErrand) -> Option<SiteSignature> {
    live.the_signature_consented()?.through_the_server?.signed
}
