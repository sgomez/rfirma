//! Contexto `identity` (ADR-0017): la raíz de composición y lo que presta a los vecinos.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use application::certificates::ListedCertificates;
use domain::algorithm::SignatureAlgorithm;
use domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use domain::error::TokenError;
use domain::store::Store;
use ports::{CertificateMemory, Token};

/// La raíz de `identity`: el token, los almacenes, el listado vivo y el certificado recordado.
pub struct IdentityRoot {
    /// El token por el que se lista y se firma.
    pub token: Box<dyn Token + Send + Sync>,
    /// Almacenes de certificados configurados.
    pub stores: Vec<Store>,
    /// Directorio de certificados de software instalados.
    pub installed_certificates: PathBuf,
    /// Certificados del último listado.
    pub listed: ListedCertificates,
    /// Donde se recuerda el certificado con el que se firmó.
    pub memory: Arc<dyn CertificateMemory + Send + Sync>,
    /// La carpeta donde vive cada `.p12` instalado.
    pub folder: Arc<dyn ports::InstalledFolder + Send + Sync>,
}

impl IdentityRoot {
    /// Todos los almacenes, incluidos los de los `.p12` instalados.
    pub fn all_stores(&self) -> Vec<Store> {
        let mut stores = self.stores.clone();
        if let Some(softoken) = adapters::pkcs11::stores::softoken() {
            stores.extend(adapters::pkcs11::stores::installed_stores(
                &softoken,
                &self.installed_certificates,
            ));
        }
        stores
    }

    /// Los certificados de todos los almacenes, o por qué ninguno se ha podido abrir.
    pub fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        self.token.list_across(&self.all_stores())
    }

    /// Las filas con su asa acuñada y el recordado marcado.
    pub fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        application::certificates::rows_of(
            found,
            &self.installed_certificates,
            &self.listed,
            self.memory.as_ref(),
        )
    }

    /// El certificado de la última búsqueda tras el asa, si sigue en el token y está vigente.
    pub fn usable<'a>(
        &self,
        found: &'a [TokenCertificate],
        handle: &str,
    ) -> Result<&'a TokenCertificate, TokenError> {
        application::certificates::usable_certificate(found, handle, &self.listed)
    }

    /// El certificado del asa, listado de nuevo y comprobado antes de usarlo.
    pub fn chosen(&self, handle: &str) -> Result<TokenCertificate, TokenError> {
        let found = self.certificates()?;
        self.usable(&found, handle).cloned()
    }

    /// Apunta el certificado con el que se acaba de firmar.
    pub fn remember_the_certificate(&self, reference: &CertificateRef) {
        application::certificates::remember_the_certificate(self.memory.as_ref(), reference);
    }

    /// El certificado recordado entre sesiones, si lo hay.
    pub fn remembered_certificate(&self) -> Option<CertificateRef> {
        self.memory.remembered_certificate()
    }

    /// Olvida el certificado recordado.
    pub fn forget_the_certificate(&self) {
        application::certificates::forget_the_certificate(self.memory.as_ref());
    }

    /// El directorio de los `.p12` instalados.
    pub fn installed_certificates(&self) -> &Path {
        &self.installed_certificates
    }

    /// El token visto por el ciclo de firma: pide el secreto y firma, nada más.
    pub fn signer(&self) -> impl crate::signing::ports::Signer + '_ {
        TokenSigner(self.token.as_ref())
    }
}

struct TokenSigner<'a>(&'a (dyn Token + Send + Sync));

impl crate::signing::ports::Signer for TokenSigner<'_> {
    fn secret_of(
        &self,
        reference: &CertificateRef,
    ) -> Result<domain::secret::StoreSecret, TokenError> {
        self.0.secret_of(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        self.0.offers(reference, algorithm)
    }

    fn sign(
        &self,
        reference: &CertificateRef,
        pin: &str,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        self.0.sign(reference, pin, algorithm, data)
    }
}

impl<T: ports::Token + ?Sized> crate::signing::ports::Signer for T {
    fn secret_of(
        &self,
        reference: &domain::certificate::CertificateRef,
    ) -> Result<domain::secret::StoreSecret, domain::error::TokenError> {
        ports::Token::secret_of(self, reference)
    }

    fn offers(
        &self,
        reference: &domain::certificate::CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), domain::error::TokenError> {
        ports::Token::offers(self, reference, algorithm)
    }

    fn sign(
        &self,
        reference: &domain::certificate::CertificateRef,
        pin: &str,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, domain::error::TokenError> {
        ports::Token::sign(self, reference, pin, algorithm, data)
    }
}
