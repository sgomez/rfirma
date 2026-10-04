//! El certificado de `-certtui`: elegido en la terminal entre los vigentes que dejan `-store` y `-filter`, una fila por certificado y con el recordado delante; no pinta la lista.

use std::time::Duration;

use x509_cert::der::DateTime;

use super::{accepted_by, listed_within_the_store, the_site_filter_of, CommandLinePorts, Outcome};
use crate::desktop::ports::OfferedCertificate;
use crate::identity::domain::certificate::{CertificateRef, CertificateStatus, TokenCertificate};
use crate::identity::domain::copies::{copies_of_each_certificate, ChosenCopy};
use crate::identity::domain::holder::{
    common_name_of, given_name_and_surname, holder_of, is_representative, without_semantics_prefix,
};
use crate::identity::domain::store::StoreClass;

pub(super) fn the_certificate_chosen_on_the_terminal(
    filter: Option<&str>,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<TokenCertificate, Outcome> {
    if !ports.terminal.is_interactive() {
        return Err(Outcome::failed(
            "rfirma: --certtui necesita una terminal en la que elegir el certificado; \
             sin ella, usa --alias o --filter"
                .to_owned(),
        ));
    }
    let rows = the_rows(the_usable_certificates(filter, arguments, ports)?, ports);
    let preselected = rows.iter().position(|row| row.copy.remembered).unwrap_or(0);
    let offered: Vec<OfferedCertificate> = rows.iter().map(|row| row.offered.clone()).collect();
    let index = ports
        .terminal
        .chosen(&offered, preselected)
        .map_err(|reason| {
            Outcome::failed(format!(
                "rfirma: no se ha elegido ningún certificado ({reason})"
            ))
        })?;
    rows.into_iter()
        .nth(index)
        .map(|row| row.copy.certificate)
        .ok_or_else(|| {
            Outcome::failed(
                "rfirma: la terminal ha elegido un certificado que no estaba en la lista"
                    .to_owned(),
            )
        })
}

/// Un certificado de la lista, con la copia con la que firma.
struct Row {
    copy: ChosenCopy,
    offered: OfferedCertificate,
}

/// Una fila por certificado, ordenadas como el desplegable de la ventana.
fn the_rows(usable: Vec<TokenCertificate>, ports: &CommandLinePorts) -> Vec<Row> {
    let remembered = ports.signer.remembered();
    let class_of = |reference: &CertificateRef| ports.stores.class_of(reference);
    let mut rows: Vec<Row> = copies_of_each_certificate(usable)
        .into_iter()
        .map(|copies| {
            let stores = stores_of(&copies, class_of);
            let copy = ChosenCopy::among(copies, class_of, remembered.as_ref());
            let offered = offered(&copy.certificate, stores);
            Row { copy, offered }
        })
        .collect();
    rows.sort_by(|a, b| {
        a.offered
            .headline
            .to_lowercase()
            .cmp(&b.offered.headline.to_lowercase())
            .then_with(|| a.offered.stores.cmp(&b.offered.stores))
    });
    rows
}

fn stores_of(
    copies: &[TokenCertificate],
    class_of: impl Fn(&CertificateRef) -> StoreClass,
) -> Vec<String> {
    let mut stores: Vec<(StoreClass, String)> = copies
        .iter()
        .map(|copy| {
            let class = class_of(copy.reference());
            (class, store_of(class, copy.reference()))
        })
        .collect();
    stores.sort_by_key(|(class, _)| class.preference());
    let mut names: Vec<String> = stores.into_iter().map(|(_, name)| name).collect();
    names.dedup();
    names
}

fn the_usable_certificates(
    filter: Option<&str>,
    arguments: &[String],
    ports: &CommandLinePorts,
) -> Result<Vec<TokenCertificate>, Outcome> {
    let filter = filter.map(the_site_filter_of).transpose()?;
    let mut listed = listed_within_the_store(arguments, ports)?;
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

fn offered(certificate: &TokenCertificate, stores: Vec<String>) -> OfferedCertificate {
    let subject = certificate.subject();
    let (holder_name, id_number) = holder_of(subject.as_deref());
    let id_number = without_semantics_prefix(&id_number);
    let (given_name, surname) = given_name_and_surname(subject.as_deref());
    let organization_identifier = certificate.organization_identifier();
    let entity_name = is_representative(organization_identifier.as_deref(), &given_name, &surname)
        .then(|| certificate.organization_name())
        .flatten();
    let (headline, capacity) = match entity_name {
        Some(entity) => {
            let representative = [given_name.as_str(), surname.as_str()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let headline = match organization_identifier.as_deref() {
                Some(identifier) => format!("{entity} · {}", without_semantics_prefix(identifier)),
                None => entity,
            };
            (
                headline,
                joined(&["Representante", &representative, id_number]),
            )
        }
        None => (holder_name, joined(&["A título personal", id_number])),
    };
    OfferedCertificate {
        headline,
        capacity,
        issuer: common_name_of(certificate.issuer().as_deref()),
        expires: expiry_of(&certificate.status()),
        stores,
    }
}

fn joined(parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ")
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
