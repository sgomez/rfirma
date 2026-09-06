//! Andamios y datos de prueba compartidos entre casos de uso.

use std::path::Path;

use crate::desktop::adapters::paths::Paths;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::signing::adapters::orders::{PlacementOrder, SigningOrder, VisibleFieldsOrder};
use crate::Memory;

/// Construye un certificado de prueba con la etiqueta y DER proporcionados.
pub(crate) fn a_certificate(label: &str, der: &[u8]) -> TokenCertificate {
    a_certificate_with_id(label, 0x01, der)
}

/// Construye un certificado de prueba especificando su CKA_ID.
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

/// Construye un certificado X.509 válido generado con la CA local de pruebas.
pub(crate) fn a_usable_certificate(label: &str) -> TokenCertificate {
    let ca =
        crate::site::adapters::tls::LocalCa::generate().expect("la CA local deberia generarse");
    let der = ca
        .certificate()
        .to_der()
        .expect("el certificado deberia poder salir en DER");
    a_certificate(label, &der)
}

/// Inicializa un registro de certificados listados y devuelve sus identificadores.
pub(crate) fn listed_from(certificates: &[TokenCertificate]) -> (ListedCertificates, Vec<String>) {
    let listed = ListedCertificates::new();
    let handles = listed.replace(
        certificates
            .iter()
            .map(|certificate| certificate.reference().clone()),
    );
    (listed, handles)
}

/// Crea una memoria aislada bajo la ruta temporal indicada.
pub(crate) fn a_memory(root: &Path) -> Memory {
    Memory::at(&Paths::under(root))
}

/// Genera una orden de firma completa de partida para pruebas.
pub(crate) fn an_order() -> SigningOrder {
    SigningOrder {
        document: "/run/user/1000/doc/1e8b83b9/contrato.pdf".to_owned(),
        certificate: "FIRMA".to_owned(),
        placement: Some(PlacementOrder {
            page: 1,
            pages: crate::signing::domain::PageSet::only_page(1),
            page_count: 3,
            media_box: [0.0, 0.0, 595.0, 842.0],
            rotation: 0,
            rect: [72.0, 500.0, 272.0, 600.0],
        }),
        fields: VisibleFieldsOrder {
            signer_name: true,
            issuer: true,
            signed_at: true,
            reason: true,
        },
        reason: String::new(),
        signed_at: "31/08/26, 12:00:00".to_owned(),
        rubric: None,
        language: "es".to_owned(),
        allow_unregistered_signatures: false,
    }
}
