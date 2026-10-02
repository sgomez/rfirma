//! El certificado de `-certtui`: elegido en la terminal entre los vigentes que dejan `-store` y `-filter`, con el recordado delante; no pinta la lista.

use std::time::Duration;

use x509_cert::der::DateTime;

use super::{accepted_by, listed_within_the_store, the_site_filter_of, CommandLinePorts, Outcome};
use crate::desktop::ports::{CertificateStores, OfferedCertificate};
use crate::identity::domain::certificate::{CertificateRef, CertificateStatus, TokenCertificate};
use crate::identity::domain::holder::common_name_of;
use crate::identity::domain::store::StoreClass;

pub(super) fn the_certificate_chosen_on_the_terminal(
    filter: Option<&str>,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<TokenCertificate, Outcome> {
    if !ports.terminal.is_interactive() {
        return Err(Outcome::failed(
            "rfirma: -certtui necesita una terminal en la que elegir el certificado; \
             sin ella, usa -alias o -filter"
                .to_owned(),
        ));
    }
    let usable = the_usable_certificates(filter, arguments, ports)?;
    let preselected = position_of_the_remembered(&usable, ports.signer.remembered().as_ref());
    let offered: Vec<OfferedCertificate> = usable
        .iter()
        .map(|certificate| offered(certificate, ports.stores))
        .collect();
    let index = ports
        .terminal
        .chosen(&offered, preselected)
        .map_err(|reason| {
            Outcome::failed(format!(
                "rfirma: no se ha elegido ningún certificado ({reason})"
            ))
        })?;
    usable.get(index).cloned().ok_or_else(|| {
        Outcome::failed(
            "rfirma: la terminal ha elegido un certificado que no estaba en la lista".to_owned(),
        )
    })
}

fn the_usable_certificates(
    filter: Option<&str>,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<Vec<TokenCertificate>, Outcome> {
    let filter = filter.map(the_site_filter_of).transpose()?;
    let mut listed = listed_within_the_store(arguments, ports.stores)?;
    if let Some(filter) = &filter {
        listed = accepted_by(filter, listed, ports)?;
    }
    listed.retain(|certificate| certificate.status().is_usable());
    if listed.is_empty() {
        return Err(Outcome::failed(
            "rfirma: no hay ningún certificado vigente que elegir".to_owned(),
        ));
    }
    Ok(listed)
}

fn position_of_the_remembered(
    usable: &[TokenCertificate],
    remembered: Option<&CertificateRef>,
) -> usize {
    remembered
        .and_then(|remembered| {
            usable
                .iter()
                .position(|certificate| certificate.reference().is_the_same_as(remembered))
        })
        .unwrap_or(0)
}

fn offered(certificate: &TokenCertificate, stores: &dyn CertificateStores) -> OfferedCertificate {
    OfferedCertificate {
        holder: common_name_of(certificate.subject().as_deref()),
        issuer: common_name_of(certificate.issuer().as_deref()),
        expires: expiry_of(&certificate.status()),
        store: store_of(
            stores.class_of(certificate.reference()),
            certificate.reference(),
        ),
    }
}

fn expiry_of(status: &CertificateStatus) -> String {
    let CertificateStatus::Valid { not_after } = status else {
        return String::new();
    };
    DateTime::from_unix_duration(Duration::from_secs(*not_after))
        .map(|date| format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()))
        .unwrap_or_default()
}

pub(super) fn store_of(class: StoreClass, reference: &CertificateRef) -> String {
    match class {
        StoreClass::Card => format!("tarjeta «{}»", reference.token_label()),
        StoreClass::Installed => "Almacén de rFirma".to_owned(),
        StoreClass::Firefox => "Firefox".to_owned(),
        StoreClass::Chrome => "Chrome".to_owned(),
        StoreClass::Nssdb => "NSS del sistema".to_owned(),
        StoreClass::Windows => "Windows".to_owned(),
    }
}
