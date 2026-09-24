//! La firma de sede sobre un PDF cifrado: su contraseña se pide a la persona, salvo con `headless`.

use crate::identity::domain::secret::StoreSecret;
use crate::signing::domain::{unlocked_with, Refusal, Waivers};
use crate::site::application::session::{self as signing, SiteRefusal, SiteTerms};
use crate::site::ports::{FilterEngine, PolicyEngine};

use super::desk::{ErrandDesk, Neighbours};
use super::state::PendingSignature;

/// Por qué no se abrió la firma.
#[derive(Debug)]
pub(super) enum Unopened {
    /// El trámite se rechaza por esto.
    Refused(SiteRefusal),
    /// La persona no dio la contraseña del PDF.
    Declined,
}

/// Abre la firma, pidiendo la contraseña del PDF hasta que lo abra o la persona desista.
pub(super) fn begun_with_the_pdf_password<E: FilterEngine, P: PolicyEngine, N: Neighbours>(
    desk: &ErrandDesk<'_, E, P, N>,
    pending: &PendingSignature,
    certificate: &str,
) -> Result<StoreSecret, Unopened> {
    let mut from_the_site = pending.from_the_site.clone();
    let mut after_a_wrong_one = Waivers::declared_in(
        from_the_site
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str())),
    )
    .declares_a_password();
    loop {
        let begun = signing::begin_for_the_site(
            &SiteTerms {
                engine: desk.engine,
                filter: &pending.filter,
                format: pending.format,
                algorithm: pending.algorithm,
                operation: pending.operation,
                from_the_site: &from_the_site,
                allow_unregistered_signatures: pending.unregistered_signatures,
            },
            &pending.document,
            certificate,
            &desk.neighbours,
            &desk.neighbours,
        );
        match begun {
            Err(SiteRefusal::Signing(refusal)) if refusal.awaits_the_pdf_password() => {}
            other => return other.map_err(Unopened::Refused),
        }
        if pending.headless {
            return Err(Unopened::Refused(SiteRefusal::ConfirmationNeeded(
                Refusal::Encrypted.situation().to_owned(),
            )));
        }
        let Some(password) = desk.neighbours.the_pdf_password(after_a_wrong_one) else {
            return Err(Unopened::Declined);
        };
        from_the_site = unlocked_with(&from_the_site, &password);
        after_a_wrong_one = true;
    }
}
