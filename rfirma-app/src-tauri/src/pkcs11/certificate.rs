//! El certificado tal y como sale del token, y su clasificación.
//!
//! Dos ideas mandan aquí:
//!
//! - **Del certificado se guarda cómo volver a encontrarlo, no quién es**
//!   (ID-32, ADR-0010). Eso es [`CertificateRef`]: módulo, token, etiqueta y
//!   `CKA_ID`, y nada más. El titular se lee del DER cada vez que hace falta
//!   pintarlo, con
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
    /// El `CKA_ID` del certificado, que es lo que de verdad lo empareja con su
    /// clave privada.
    ///
    /// `None` es **un certificado recordado antes del #98**, cuando la
    /// referencia solo tenía tres coordenadas: el fichero de estado de una
    /// versión anterior se lee sin romperse, y lo que queda sin saber es el
    /// `CKA_ID`, no el certificado.
    #[serde(default)]
    cka_id: Option<Vec<u8>>,
}

impl CertificateRef {
    /// Construye la referencia a partir de sus cuatro coordenadas.
    ///
    /// El `CKA_ID` admite tanto `vec![0x01]` como `None`; lo segundo solo tiene
    /// sentido para reconstruir una referencia recordada por una versión
    /// anterior, porque un certificado leído del token siempre trae el suyo.
    pub fn new(
        module: impl Into<PathBuf>,
        token_label: impl Into<String>,
        label: impl Into<String>,
        cka_id: impl Into<Option<Vec<u8>>>,
    ) -> Self {
        Self {
            module: module.into(),
            token_label: token_label.into(),
            label: label.into(),
            cka_id: cka_id.into(),
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

    /// `CKA_LABEL` del objeto dentro del token. Sirve para pintarlo y para
    /// reconocerlo, **no** para encontrar su clave: en un almacén de verdad la
    /// etiqueta se repite (ID-06).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// `CKA_ID` del objeto dentro del token: la coordenada con la que PKCS#11
    /// —y NSS— emparejan certificado y clave privada.
    ///
    /// `None` solo aparece leyendo el estado de una versión anterior al #98, o
    /// en un token que no le ponga `CKA_ID` a sus objetos.
    pub fn cka_id(&self) -> Option<&[u8]> {
        self.cka_id.as_deref()
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

    /// La autoridad que lo emitió, para pintarla. **No es el `O=` del titular**:
    /// ese es la organización de quien firma, y enseñarlo como emisor le dice a
    /// quien firma que su propio organismo emitió el certificado. El emisor es
    /// el dato con el que se decide si un certificado es de fiar, así que sale
    /// del campo que de verdad lo lleva.
    ///
    /// Se recalcula del DER cada vez, por lo mismo que [`Self::subject`].
    pub fn issuer(&self) -> Option<String> {
        Certificate::from_der(&self.der)
            .ok()
            .map(|certificate| certificate.tbs_certificate().issuer().to_string())
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
    fn a_reference_carries_the_four_coordinates_and_nothing_else() {
        let reference =
            CertificateRef::new("/usr/lib/x.so", "rfirma-test", "ETIQUETA", vec![0x2a, 0x01]);

        assert_eq!(reference.module(), Path::new("/usr/lib/x.so"));
        assert_eq!(reference.token_label(), "rfirma-test");
        assert_eq!(reference.label(), "ETIQUETA");
        assert_eq!(reference.cka_id(), Some([0x2a, 0x01].as_slice()));
    }

    /// Lo que escribio una version anterior al #98 no lleva `cka_id`, y leerlo
    /// tiene que dar una referencia sin `CKA_ID` en vez de un error: el fichero
    /// de estado de quien actualiza no puede tumbar el arranque.
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
    fn a_revocation_is_not_a_token_failure() {
        let status = CertificateStatus::Revoked {
            reason: "superseded".to_owned(),
        };

        assert!(!status.is_usable());
    }
}
