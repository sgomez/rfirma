//! La ventana de sede cuando cambian las tarjetas: el consentimiento vuelve a cribar con el filtro de la sede, y sin certificado se vuelve a buscar (ADR-0048).

use crate::identity::domain::certificate::TokenCertificate;
use crate::site::application::filtering;
use crate::site::ports::{FilterEngine, PolicyEngine};

use super::{look_again, ErrandDesk, ErrandStep, LiveErrand, Moment};

/// Lo que cambia en el trámite de sede tras cambiar una tarjeta.
#[derive(Debug)]
pub enum AfterTheReaders {
    /// El consentimiento sigue abierto: lo que la sede acepta de los certificados de ahora.
    Accepted(Vec<TokenCertificate>),
    /// No había certificado: el trámite ha vuelto a mirar, y este es el paso que ha dado.
    LookedAgain(Option<ErrandStep>),
    /// Ningún momento que enseñe una lista.
    Unchanged,
}

/// Lo que el trámite hace con los certificados de ahora, tras cambiar una tarjeta.
pub fn after_the_readers<E: FilterEngine, P: PolicyEngine>(
    desk: &ErrandDesk<'_, E, P>,
    found: Vec<TokenCertificate>,
    live: &LiveErrand,
) -> AfterTheReaders {
    match live.moment() {
        Some(Moment::NoCertificate { .. }) => AfterTheReaders::LookedAgain(look_again(desk, live)),
        Some(
            Moment::AskingForConsent { .. }
            | Moment::AskingToSign { .. }
            | Moment::AskingToSignTheBatch { .. }
            | Moment::AskingToSignTheLocalBatch { .. },
        ) => live
            .the_filter_of_the_list()
            .and_then(|filter| {
                filtering::keep_what_the_site_accepts(desk.engine, &filter, found, desk.neighbours)
                    .ok()
            })
            .map_or(AfterTheReaders::Unchanged, AfterTheReaders::Accepted),
        _ => AfterTheReaders::Unchanged,
    }
}
