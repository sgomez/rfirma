//! Los dobles de los puertos de `identity` y los certificados de prueba que comparten las gradas A de todos los contextos.

use std::path::Path;

use crate::identity::application::certificates::ListedCertificates;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::Store;
use crate::identity::ports::{CertificateMemory, Token};
use crate::memory_error::MemoryError;
use crate::site::domain::local_ca::{generate_key, random_serial, LocalCa};
use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::{X509Name, X509NameRef, X509};

/// Un token sin certificados que no sabe firmar: cada almacén está vacío.
pub(crate) struct NoToken;

impl Token for NoToken {
    fn list(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn every_certificate(&self, _store: &Store) -> Result<Vec<TokenCertificate>, TokenError> {
        Ok(Vec::new())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::NotNeeded)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Ok(())
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        _pin: &str,
        _algorithm: SignatureAlgorithm,
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

    fn forget_the_certificate(&self) -> Result<(), MemoryError> {
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

/// Una autoridad de pruebas: un certificado de verdad y la clave con la que emite los que cuelgan de él.
pub(crate) struct TestAuthority {
    certificate: X509,
    key: PKey<Private>,
}

impl TestAuthority {
    /// Una raíz autofirmada con ese nombre común.
    pub(crate) fn root(common_name: &str) -> Self {
        Self::built(common_name, None)
    }

    /// Otra autoridad emitida por esta, con ese nombre común.
    pub(crate) fn issues(&self, common_name: &str) -> Self {
        Self::built(common_name, Some(self))
    }

    /// El certificado en DER.
    pub(crate) fn der(&self) -> Vec<u8> {
        self.certificate
            .to_der()
            .expect("el certificado de pruebas deberia salir en DER")
    }

    /// El certificado como certificado del token, con esa etiqueta.
    pub(crate) fn as_certificate(&self, label: &str) -> TokenCertificate {
        a_certificate(label, &self.der())
    }

    fn built(common_name: &str, issuer: Option<&Self>) -> Self {
        let key = generate_key().expect("la clave de pruebas deberia generarse");
        let mut name = X509Name::builder().expect("deberia poder construirse un nombre");
        name.append_entry_by_nid(Nid::COMMONNAME, common_name)
            .expect("el nombre comun deberia entrar");
        let name = name.build();
        let issuer_name: &X509NameRef = match issuer {
            Some(authority) => authority.certificate.subject_name(),
            None => &name,
        };

        let mut builder = X509::builder().expect("deberia poder construirse un certificado");
        builder.set_version(2).expect("la version deberia ponerse");
        builder
            .set_serial_number(&random_serial().expect("el serie deberia generarse"))
            .expect("el serie deberia ponerse");
        builder
            .set_subject_name(&name)
            .expect("el titular deberia ponerse");
        builder
            .set_issuer_name(issuer_name)
            .expect("el emisor deberia ponerse");
        builder.set_pubkey(&key).expect("la clave deberia ponerse");
        builder
            .set_not_before(&Asn1Time::days_from_now(0).expect("deberia haber fecha"))
            .expect("el inicio deberia ponerse");
        builder
            .set_not_after(&Asn1Time::days_from_now(30).expect("deberia haber fecha"))
            .expect("el fin deberia ponerse");
        builder
            .sign(
                issuer.map_or(&key, |authority| &authority.key),
                MessageDigest::sha256(),
            )
            .expect("el certificado de pruebas deberia firmarse");

        Self {
            certificate: builder.build(),
            key,
        }
    }
}
