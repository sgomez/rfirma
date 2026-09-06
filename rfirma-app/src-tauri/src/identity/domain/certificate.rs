//! Certificados leídos del token PKCS#11 y su clasificación (ADR-0010).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use x509_cert::der::Decode;
use x509_cert::Certificate;

use crate::identity::domain::store::Store;

/// Coordenadas de persistencia para reencontrar un certificado en el almacén (ADR-0010).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateRef {
    module: PathBuf,
    token_label: String,
    label: String,
    /// CKA_ID del certificado en el token.
    #[serde(default)]
    cka_id: Option<Vec<u8>>,
    /// Parámetros de inicialización requeridos por el módulo.
    #[serde(default)]
    init_args: Option<String>,
}

impl CertificateRef {
    /// Construye una referencia a partir del almacén y sus coordenadas identificadoras.
    pub fn new(
        store: impl Into<Store>,
        token_label: impl Into<String>,
        label: impl Into<String>,
        cka_id: impl Into<Option<Vec<u8>>>,
    ) -> Self {
        let store = store.into();
        Self {
            module: store.path().to_path_buf(),
            token_label: token_label.into(),
            label: label.into(),
            cka_id: cka_id.into(),
            init_args: store.init_args().map(str::to_owned),
        }
    }

    /// Almacén PKCS#11 de procedencia.
    pub fn store(&self) -> Store {
        Store::with_init_args(&self.module, self.init_args.clone())
    }

    /// Ruta del módulo PKCS#11 que lo sirve.
    pub fn module(&self) -> &Path {
        &self.module
    }

    /// Etiqueta del token dentro del módulo.
    pub fn token_label(&self) -> &str {
        &self.token_label
    }

    /// CKA_LABEL del certificado dentro del token.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// CKA_ID del objeto en el token.
    pub fn cka_id(&self) -> Option<&[u8]> {
        self.cka_id.as_deref()
    }

    /// Comprueba si dos referencias identifican al mismo certificado.
    pub fn is_the_same_as(&self, other: &Self) -> bool {
        self.module == other.module
            && self.token_label == other.token_label
            && self.label == other.label
            && agree(self.cka_id.as_deref(), other.cka_id.as_deref())
            && agree(self.init_args.as_deref(), other.init_args.as_deref())
    }
}

fn agree<T: PartialEq + ?Sized>(one: Option<&T>, other: Option<&T>) -> bool {
    match (one, other) {
        (Some(one), Some(other)) => one == other,
        _ => true,
    }
}

/// Estado de validez temporal o revocación del certificado.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateStatus {
    /// En vigor temporalmente.
    Valid { not_after: u64 },
    /// Certificado caducado.
    Expired { not_after: u64 },
    /// Certificado aún no válido.
    NotYetValid { not_before: u64 },
    /// Certificado revocado.
    Revoked { reason: String },
    /// Certificado con formato no analizable.
    Unreadable { detail: String },
}

impl CertificateStatus {
    /// Indica si el certificado es apto para firmar.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }
}

/// Certificado extraído del token junto con sus coordenadas persistibles.
#[derive(Clone, Debug)]
pub struct TokenCertificate {
    reference: CertificateRef,
    der: Vec<u8>,
}

impl TokenCertificate {
    /// Construye una instancia a partir de la referencia y el contenido DER.
    pub fn new(reference: CertificateRef, der: Vec<u8>) -> Self {
        Self { reference, der }
    }

    /// Coordenadas persistibles del certificado.
    pub fn reference(&self) -> &CertificateRef {
        &self.reference
    }

    /// Contenido en formato DER.
    pub fn der(&self) -> &[u8] {
        &self.der
    }

    /// Titular del certificado (ADR-0010).
    pub fn subject(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().subject().to_string())
    }

    /// Autoridad emisora del certificado.
    pub fn issuer(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().issuer().to_string())
    }

    /// Estado del certificado en el instante actual.
    pub fn status(&self) -> CertificateStatus {
        self.status_at(SystemTime::now())
    }

    /// Estado del certificado en un instante determinado.
    pub fn status_at(&self, instant: SystemTime) -> CertificateStatus {
        let certificate = match Certificate::from_der(&self.der) {
            Ok(certificate) => certificate,
            Err(error) => {
                return CertificateStatus::Unreadable {
                    detail: error.to_string(),
                }
            }
        };

        let validity = certificate.tbs_certificate().validity();
        let not_before = validity.not_before.to_unix_duration();
        let not_after = validity.not_after.to_unix_duration();
        let now = instant.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);

        if now > not_after {
            CertificateStatus::Expired {
                not_after: not_after.as_secs(),
            }
        } else if now < not_before {
            CertificateStatus::NotYetValid {
                not_before: not_before.as_secs(),
            }
        } else {
            CertificateStatus::Valid {
                not_after: not_after.as_secs(),
            }
        }
    }
}

#[cfg(test)]
mod tests;
