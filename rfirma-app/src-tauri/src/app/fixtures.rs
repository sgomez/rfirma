//! Los andamios que comparten las pruebas de los casos de uso.
//!
//! Existe para no repetirlos en cuatro ficheros: un certificado del token, un
//! registro de asas ya listadas, una orden de firma completa y una memoria
//! sobre un directorio temporal. Es `#[cfg(test)]` entero, así que no llega al
//! binario.

use std::path::Path;

use crate::commands::orders::{PlacementOrder, SigningOrder, VisibleFieldsOrder};
use crate::memory::{ListedCertificates, Memory};
use crate::paths::Paths;
use crate::pkcs11::{CertificateRef, TokenCertificate};

/// Un certificado del token con el DER que se le dé. Con basura dentro el
/// estado sale `Unreadable`, que es justo lo que hace falta para probar la
/// negativa sin fabricar un X.509.
pub(crate) fn a_certificate(label: &str, der: &[u8]) -> TokenCertificate {
    a_certificate_with_id(label, 0x01, der)
}

/// El mismo, con el `CKA_ID` a la vista: es lo único que distingue dos
/// certificados que comparten etiqueta.
pub(crate) fn a_certificate_with_id(label: &str, cka_id: u8, der: &[u8]) -> TokenCertificate {
    TokenCertificate::new(
        CertificateRef::new(
            "/usr/lib/softhsm/libsofthsm2.so",
            "rfirma-test",
            label,
            vec![cka_id],
        ),
        der.to_vec(),
    )
}

/// Un registro con esos certificados ya listados, y sus asas.
pub(crate) fn listed_from(certificates: &[TokenCertificate]) -> (ListedCertificates, Vec<String>) {
    let listed = ListedCertificates::new();
    let handles = listed.replace(
        certificates
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    (listed, handles)
}

/// Una memoria cuyos dos ficheros cuelgan de un directorio temporal.
pub(crate) fn a_memory(root: &Path) -> Memory {
    Memory::at(&Paths::under(root))
}

/// La orden de firma completa que sirve de punto de partida a las pruebas.
pub(crate) fn an_order() -> SigningOrder {
    SigningOrder {
        document: "/run/user/1000/doc/1e8b83b9/contrato.pdf".to_owned(),
        certificate: "FIRMA".to_owned(),
        placement: PlacementOrder {
            page: 1,
            media_box: [0.0, 0.0, 595.0, 842.0],
            rotation: 0,
            rect: [72.0, 500.0, 272.0, 600.0],
        },
        fields: VisibleFieldsOrder {
            signer_name: true,
            id_number: true,
            signed_at: true,
            reason: true,
        },
        reason: String::new(),
        signed_at: "31/08/26, 12:00:00".to_owned(),
        rubric: None,
        language: "es".to_owned(),
    }
}
