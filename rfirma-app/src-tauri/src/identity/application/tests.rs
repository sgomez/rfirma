//! Los dobles de los puertos de `identity` y los certificados de prueba que comparten las gradas A de todos los contextos.

use std::path::Path;

use crate::identity::application::certificates::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::{CertificateMemory, Token};
use crate::memory_error::MemoryError;
use crate::site::domain::local_ca::LocalCa;

/// Un token sin certificados que no sabe firmar: cada almacén está vacío.
pub(crate) struct NoToken;

impl Token for NoToken {
    fn list(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::NotNeeded)
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        Err(TokenError::new(
            Situation::CertificateNotFound,
            "este token no tiene con que firmar",
        ))
    }

    fn import_pkcs12(
        &self,
        _directory: &Path,
        _pkcs12: &[u8],
        _password: &str,
    ) -> Result<Store, TokenError> {
        Err(TokenError::new(
            Situation::Pkcs12Unreadable,
            "este token no importa nada",
        ))
    }
}

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
    let ca = LocalCa::generate().expect("la CA local deberia generarse");
    let der = ca
        .certificate()
        .to_der()
        .expect("el certificado deberia poder salir en DER");
    a_certificate(label, &der)
}

/// Una memoria que no recuerda ningún certificado y no escribe en ningún sitio.
pub(crate) struct NoMemory;

impl CertificateMemory for NoMemory {
    fn remembered_certificate(&self) -> Option<CertificateRef> {
        None
    }

    fn remember_the_certificate(&self, _reference: &CertificateRef) -> Result<(), MemoryError> {
        Ok(())
    }
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
