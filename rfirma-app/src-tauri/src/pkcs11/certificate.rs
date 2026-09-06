//! Certificados leídos del token PKCS#11 y su clasificación (ADR-0010).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use x509_cert::der::Decode;
use x509_cert::Certificate;

use super::stores::Store;

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
mod tests {
    use super::*;

    #[test]
    fn a_reference_carries_the_four_coordinates_and_nothing_else() {
        let reference =
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x2a, 0x01]);

        assert_eq!(reference.module(), Path::new("/usr/lib/x.so"));
        assert_eq!(reference.token_label(), "rfirma-test");
        assert_eq!(reference.label(), "ETIQUETA");
        assert_eq!(reference.cka_id(), Some([0x2a, 0x01].as_slice()));
    }

    #[test]
    fn a_reference_remembered_before_the_cka_id_existed_still_reads() {
        let written = r#"{
            "module": "/usr/lib/x.so",
            "token_label": "rfirma-test",
            "label": "ETIQUETA"
        }"#;

        let reference: CertificateRef =
            serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

        assert_eq!(reference.label(), "ETIQUETA");
        assert_eq!(reference.cka_id(), None);
    }

    #[test]
    fn a_reference_round_trips_through_the_state_file_with_its_cka_id() {
        let reference = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x05]);

        let written = serde_json::to_string(&reference).expect("deberia serializarse");
        let read: CertificateRef = serde_json::from_str(&written).expect("deberia leerse");

        assert_eq!(read, reference);
        assert_eq!(read.cka_id(), Some([0x05].as_slice()));
    }

    #[test]
    fn a_der_that_is_not_a_certificate_is_unreadable_rather_than_a_panic() {
        let certificate = TokenCertificate::new(
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "BASURA", vec![0x01]),
            vec![0x00, 0x01, 0x02],
        );

        assert!(matches!(
            certificate.status(),
            CertificateStatus::Unreadable { .. }
        ));
        assert_eq!(certificate.subject(), None);
        assert_eq!(certificate.issuer(), None);
        assert!(!certificate.status().is_usable());
    }

    #[test]
    fn a_remembered_reference_recognises_the_one_that_came_out_of_the_token() {
        let remembered = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "FIRMA", vec![0x01]);

        assert!(remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "rfirma-test",
            "FIRMA",
            vec![0x01]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "rfirma-test",
            "FIRMA",
            vec![0x02]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/x.so",
            "otro-token",
            "FIRMA",
            vec![0x01]
        )));
        assert!(!remembered.is_the_same_as(&CertificateRef::new(
            "/usr/lib/otro.so",
            "rfirma-test",
            "FIRMA",
            vec![0x01]
        )));
    }

    #[test]
    fn a_reference_remembered_by_an_older_version_still_finds_its_certificate() {
        let written = r#"{
            "module": "/usr/lib/libsoftokn3.so",
            "token_label": "NSS Certificate DB",
            "label": "FIRMA"
        }"#;
        let remembered: CertificateRef =
            serde_json::from_str(written).expect("una referencia antigua tiene que leerse");

        let listed = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/home/quien/.mozilla/firefox/abc'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );

        assert!(remembered.is_the_same_as(&listed));
    }

    #[test]
    fn two_firefox_profiles_are_not_the_same_certificate() {
        let one = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/uno'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );
        let other = CertificateRef::new(
            Store::with_init_args(
                "/usr/lib/libsoftokn3.so",
                Some("configdir='/otro'".to_owned()),
            ),
            "NSS Certificate DB",
            "FIRMA",
            vec![0x01],
        );

        assert!(!one.is_the_same_as(&other));
    }

    #[test]
    fn a_revocation_is_not_a_token_failure() {
        let status = CertificateStatus::Revoked {
            reason: "superseded".to_owned(),
        };

        assert!(!status.is_usable());
    }
}
