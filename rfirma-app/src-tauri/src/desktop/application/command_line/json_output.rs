//! Los tipos del `--json` de cada orden, propios de la línea de órdenes y construidos desde el dominio, que no se serializa (ADR-0041).

use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};
use openssl::bn::BigNum;
use serde::Serialize;

use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::store::StoreClass;

/// Lo que saca `listaliases --json`.
#[derive(Serialize)]
pub(super) struct ListedAliases {
    certificates: Vec<ListedAlias>,
}

#[derive(Serialize)]
struct ListedAlias {
    alias: String,
    store: String,
    #[serde(flatten)]
    certificate: CertificateOutput,
}

/// Un certificado, igual en todas las órdenes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CertificateOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    not_before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    not_after: Option<String>,
}

impl ListedAliases {
    pub(super) fn of(certificates: &[TokenCertificate]) -> Self {
        Self {
            certificates: certificates
                .iter()
                .map(|certificate| ListedAlias {
                    alias: certificate.reference().label().to_owned(),
                    store: store_name_of(certificate),
                    certificate: CertificateOutput::of(certificate),
                })
                .collect(),
        }
    }
}

impl CertificateOutput {
    pub(super) fn of(certificate: &TokenCertificate) -> Self {
        let validity = certificate.validity();
        Self {
            subject: certificate.subject(),
            issuer: certificate.issuer(),
            serial_number: certificate
                .serial_number()
                .as_deref()
                .and_then(in_hexadecimal),
            not_before: validity.map(|(not_before, _)| in_rfc3339(not_before)),
            not_after: validity.map(|(_, not_after)| in_rfc3339(not_after)),
        }
    }
}

/// Una sola línea compacta, en UTF-8 sin escapar y con salto final.
pub(super) fn compact(output: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(output).unwrap_or_default();
    bytes.push(b'\n');
    bytes
}

fn store_name_of(certificate: &TokenCertificate) -> String {
    let reference = certificate.reference();
    match reference.store().class() {
        StoreClass::Card => format!("pkcs11:{}", reference.module().display()),
        StoreClass::Windows => "windows".to_owned(),
        StoreClass::Firefox | StoreClass::Chrome | StoreClass::Nssdb | StoreClass::Installed => {
            "mozilla".to_owned()
        }
    }
}

fn in_hexadecimal(decimal: &str) -> Option<String> {
    let hex = BigNum::from_dec_str(decimal).ok()?.to_hex_str().ok()?;
    let whole_bytes = if hex.len() % 2 == 0 { "" } else { "0" };
    Some(format!("{whole_bytes}{hex}"))
}

fn in_rfc3339(instant: SystemTime) -> String {
    DateTime::<Utc>::from(instant).to_rfc3339_opts(SecondsFormat::Secs, true)
}
