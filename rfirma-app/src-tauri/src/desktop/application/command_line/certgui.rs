//! El certificado de `-certgui`: lo elige la persona, con el PIN, en la ventana de sede con el origen «orden de terminal»; no abre la ventana.

use std::path::Path;

use super::{accepted_by, listed_within_the_store, the_site_filter_of, CommandLinePorts, Outcome};
use crate::desktop::ports::{NoCertificateToOffer, WindowChoice, WindowOffer};
use crate::identity::domain::certificate::{CertificateStatus, TokenCertificate};
use crate::identity::domain::protected_secret::ProtectedSecret;

pub(super) fn the_certificate_chosen_in_the_window(
    filter: Option<&str>,
    document: &Path,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<(TokenCertificate, Option<ProtectedSecret>), Outcome> {
    if !ports.window.has_a_display() {
        return Err(Outcome::failed(
            "rfirma: -certgui necesita un entorno gráfico en el que abrir la ventana; \
             sin él, usa -certtui, -alias o -filter"
                .to_owned(),
        ));
    }
    let offered = the_offered_certificates(filter, arguments, ports)?;
    let offer = match &offered {
        Ok(certificates) => WindowOffer::Certificates(certificates),
        Err(nothing) => WindowOffer::Nothing(*nothing),
    };
    let choice = ports.window.chosen(document, offer).map_err(|reason| {
        Outcome::failed(format!(
            "rfirma: no se puede abrir la ventana de sede ({reason})"
        ))
    })?;
    match choice {
        WindowChoice::Chosen {
            certificate,
            secret,
        } => Ok((*certificate, secret)),
        WindowChoice::Cancelled if offered.is_err() => Err(Outcome::failed(
            "rfirma: no hay ningún certificado vigente que elegir".to_owned(),
        )),
        WindowChoice::Cancelled => Err(Outcome::failed(
            "rfirma: se ha cancelado la firma en la ventana".to_owned(),
        )),
    }
}

fn the_offered_certificates(
    filter: Option<&str>,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<Result<Vec<TokenCertificate>, NoCertificateToOffer>, Outcome> {
    let filter = filter.map(the_site_filter_of).transpose()?;
    let listed = listed_within_the_store(arguments, ports.stores)?;
    let owned = listed.len();
    if owned == 0 {
        return Ok(Err(NoCertificateToOffer::None));
    }
    let mut kept = match &filter {
        Some(filter) => accepted_by(filter, listed, ports)?,
        None => listed,
    };
    if kept.is_empty() {
        return Ok(Err(NoCertificateToOffer::Excluded { owned }));
    }
    let before_dropping_the_expired = kept.len();
    kept.retain(|certificate| !matches!(certificate.status(), CertificateStatus::Expired { .. }));
    if kept.is_empty() {
        return Ok(Err(NoCertificateToOffer::AllExpired {
            owned: before_dropping_the_expired,
        }));
    }
    Ok(Ok(kept))
}
