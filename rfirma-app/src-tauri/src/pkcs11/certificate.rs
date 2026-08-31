//! El certificado tal y como sale del token, y su clasificación.
//!
//! Dos ideas mandan aquí:
//!
//! - **Del certificado se guarda cómo volver a encontrarlo, no quién es**
//!   (ID-32, ADR-0010). Eso es [`CertificateRef`]: módulo y etiqueta, y nada
//!   más. El titular se lee del DER cada vez que hace falta pintarlo, con
//!   [`TokenCertificate::subject`], y por eso no hay forma de persistirlo desde
//!   aquí sin escribirlo a propósito.
//! - **Un certificado caducado no es un fallo del token.** Es un
//!   [`CertificateStatus`], se conoce leyendo el DER —sin sesión y **sin pedir
//!   el PIN**— y por eso no comparte tipo con [`TokenError`](super::TokenError).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use x509_cert::der::Decode;
use x509_cert::Certificate;

/// Cómo volver a encontrar un certificado en el próximo arranque (ID-32).
///
/// Es lo único de esta parte del programa que tiene sentido persistir: no lleva
/// titular, ni DNI, ni número de serie. Por eso es **este** tipo el que se
/// serializa en el estado ([`crate::memory`]) y no [`TokenCertificate`], que
/// arrastra el DER entero.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateRef {
    module: PathBuf,
    token_label: String,
    label: String,
}

impl CertificateRef {
    /// Construye la referencia a partir de sus tres coordenadas.
    pub fn new(
        module: impl Into<PathBuf>,
        token_label: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            module: module.into(),
            token_label: token_label.into(),
            label: label.into(),
        }
    }

    /// Ruta del módulo PKCS#11 que lo sirve.
    pub fn module(&self) -> &Path {
        &self.module
    }

    /// Etiqueta del token dentro de ese módulo. El número de ranura **no** vale:
    /// SoftHSM lo reasigna al inicializar y una tarjeta cambia de ranura al
    /// reinsertarla.
    pub fn token_label(&self) -> &str {
        &self.token_label
    }

    /// `CKA_LABEL` del objeto dentro del token.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// En qué estado está el certificado, decidido **antes** de pedir el PIN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CertificateStatus {
    /// En vigor, hasta donde se puede saber sin red.
    Valid,
    /// Ya caducó. La fecha va en segundos desde la época.
    Expired { not_after: u64 },
    /// Todavía no ha entrado en vigor.
    NotYetValid { not_before: u64 },
    /// Revocado por su emisora.
    ///
    /// Esto **no** lo produce este módulo: comprobar la revocación es hablar con
    /// el OCSP, que es grada D y solo corre en el cron (TD-08). La variante está
    /// para que el resultado de esa comprobación tenga dónde caer sin
    /// disfrazarse de fallo del token.
    Revoked { reason: String },
    /// El DER que hay en el token no es un certificado X.509 que sepamos leer.
    Unreadable { detail: String },
}

impl CertificateStatus {
    /// Si se puede firmar con él. Lo mira el recorrido de firma antes de abrir
    /// el diálogo del PIN.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Un certificado leído del token: la referencia para reencontrarlo, y el DER
/// para todo lo demás.
#[derive(Clone, Debug)]
pub struct TokenCertificate {
    reference: CertificateRef,
    der: Vec<u8>,
}

impl TokenCertificate {
    /// Envuelve el DER tal cual sale de `CKA_VALUE`.
    pub fn new(reference: CertificateRef, der: Vec<u8>) -> Self {
        Self { reference, der }
    }

    /// Las coordenadas persistibles.
    pub fn reference(&self) -> &CertificateRef {
        &self.reference
    }

    /// El certificado en DER, tal cual está en el token.
    pub fn der(&self) -> &[u8] {
        &self.der
    }

    /// El titular, para pintarlo. Se recalcula del DER cada vez: no se almacena
    /// ni se devuelve dentro de [`CertificateRef`], porque el ADR-0010 dice que
    /// esto no se persiste.
    pub fn subject(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().subject().to_string())
    }

    /// El estado ahora mismo, leyendo el reloj del sistema.
    pub fn status(&self) -> CertificateStatus {
        self.status_at(SystemTime::now())
    }

    /// El estado en un instante dado. Existe con parámetro para poder probar la
    /// caducidad sin fabricar certificados ni tocar el reloj de la máquina.
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
            CertificateStatus::Valid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Grada A**: no hay token de por medio.
    #[test]
    fn a_reference_carries_the_three_coordinates_and_nothing_else() {
        let reference = CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA");

        assert_eq!(reference.module(), Path::new("/usr/lib/x.so"));
        assert_eq!(reference.token_label(), "rfirma-test");
        assert_eq!(reference.label(), "ETIQUETA");
    }

    #[test]
    fn a_der_that_is_not_a_certificate_is_unreadable_rather_than_a_panic() {
        let certificate = TokenCertificate::new(
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "BASURA"),
            vec![0x00, 0x01, 0x02],
        );

        assert!(matches!(
            certificate.status(),
            CertificateStatus::Unreadable { .. }
        ));
        assert_eq!(certificate.subject(), None);
        assert!(!certificate.status().is_usable());
    }

    #[test]
    fn a_revocation_is_not_a_token_failure() {
        let status = CertificateStatus::Revoked {
            reason: "superseded".to_owned(),
        };

        assert!(!status.is_usable());
    }
}
