//! Contexto `identity` (ADR-0017): la raíz de composición, `IdentityRoot`, lo que presta a los vecinos y el `Signer` de `signing` sobre cualquier `Token`.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use application::certificates::ListedCertificates;
use application::readers::{CardListing, LastListing, ReaderNow};
use domain::algorithm::SignatureAlgorithm;
use domain::certificate::{CertificateRef, ListedCertificate, TokenCertificate};
use domain::error::TokenError;
use domain::protected_secret::ProtectedSecret;
use domain::store::{Store, StoreClass};
use ports::{CertificateMemory, KeyringFactory, SecretPrompter, Token};

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
    /// La copia instalada de cada fila del último listado que tenga una, aunque no sea la elegida.
    pub installed_copies: ListedCertificates,
    /// El último listado completo, del que la lista en caliente toma lo que no es de tarjeta.
    pub last_listing: LastListing,
    /// El último estado anunciado de los lectores, para la ventana que se monta tarde.
    pub reader_now: ReaderNow,
    /// Donde se recuerda el certificado con el que se firmó.
    pub memory: Arc<dyn CertificateMemory + Send + Sync>,
    /// La carpeta donde vive cada `.p12` instalado.
    pub folder: Arc<dyn ports::InstalledFolder + Send + Sync>,
    /// El diálogo interactivo que pide la contraseña al instalar un `.p12`.
    pub prompter: Arc<dyn SecretPrompter + Send + Sync>,
    /// El llavero del escritorio con el PIN del Almacén de rFirma, alcanzado bajo demanda.
    pub keyring: KeyringFactory,
}

impl IdentityRoot {
    /// Todos los almacenes, incluidos los de los `.p12` instalados.
    pub fn all_stores(&self) -> Vec<Store> {
        every_store(self.stores.clone(), &self.installed_certificates)
    }

    /// Lo que la lista en caliente necesita para volver a listar con las tarjetas de ahora.
    pub fn card_listing(&self) -> CardListing<'_> {
        CardListing {
            token: self.token.as_ref(),
            stores: self.all_stores(),
            installed_dir: &self.installed_certificates,
            listed: &self.listed,
            installed_copies: &self.installed_copies,
            memory: self.memory.as_ref(),
            last: &self.last_listing,
        }
    }

    /// El módulo PKCS#11 descubierto que es, canonizada, la biblioteca que se nombra.
    pub fn discovered_module(&self, library: &str) -> Option<PathBuf> {
        adapters::pkcs11::stores::discovered_module_named(&self.stores, library)
    }

    /// Los certificados de todos los almacenes, o por qué ninguno se ha podido abrir.
    pub fn certificates(&self) -> Result<Vec<TokenCertificate>, TokenError> {
        application::certificates::certificates_with_their_chains(
            self.token.as_ref(),
            &self.all_stores(),
        )
    }

    /// Cuántos certificados firmables propios tiene cada clase de almacén que tenga alguno.
    pub fn certificates_by_class(&self) -> Vec<(StoreClass, usize)> {
        application::certificates::certificates_by_class(
            self.token.as_ref(),
            &self.all_stores(),
            &self.installed_certificates,
        )
    }

    /// Como `rows_of`, pero el certificado que ya tenía asa la conserva.
    pub fn rows_keeping_handles(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        application::certificates::rows_keeping_handles(
            found,
            &self.installed_certificates,
            &self.listed,
            &self.installed_copies,
            self.memory.as_ref(),
        )
    }

    /// Si el almacén es de tarjeta, para el estado del lector.
    pub fn is_a_card(&self, store: &Store) -> bool {
        application::readers::is_a_card(store, &self.installed_certificates)
    }

    /// Las filas con su asa acuñada y el recordado marcado.
    pub fn rows_of(&self, found: Vec<TokenCertificate>) -> Vec<ListedCertificate> {
        application::certificates::rows_of(
            found,
            &self.installed_certificates,
            &self.listed,
            &self.installed_copies,
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
        TokenSigner {
            token: self.token.as_ref(),
            installed_certificates: &self.installed_certificates,
            keyring: &self.keyring,
        }
    }
}

/// Los almacenes configurados y el Almacén de rFirma de ese directorio, si ya tiene algún certificado.
pub fn every_store(mut stores: Vec<Store>, installed_certificates: &Path) -> Vec<Store> {
    if let Some(softoken) = adapters::pkcs11::stores::softoken() {
        stores.extend(adapters::pkcs11::stores::installed_stores(
            &softoken,
            installed_certificates,
        ));
    }
    stores
}

/// El token de firma, con el Almacén de rFirma tomando su PIN del llavero en vez de pedirlo; a los demás almacenes no los toca.
struct TokenSigner<'a> {
    token: &'a (dyn Token + Send + Sync),
    installed_certificates: &'a Path,
    keyring: &'a KeyringFactory,
}

impl TokenSigner<'_> {
    fn is_installed(&self, reference: &CertificateRef) -> bool {
        reference
            .store()
            .installed_directory_under(self.installed_certificates)
            .is_some()
    }

    /// El PIN del llavero si el certificado es del Almacén de rFirma; si no, el secreto recibido.
    fn secret_for(
        &self,
        reference: &CertificateRef,
        provided: &ProtectedSecret,
    ) -> Result<ProtectedSecret, TokenError> {
        if !self.is_installed(reference) {
            return Ok(ProtectedSecret::new(provided.as_bytes()));
        }
        Ok((self.keyring)()?.pin()?)
    }
}

impl crate::signing::ports::Signer for TokenSigner<'_> {
    fn secret_of(
        &self,
        reference: &CertificateRef,
    ) -> Result<domain::secret::StoreSecret, TokenError> {
        if self.is_installed(reference) {
            return Ok(domain::secret::StoreSecret::NotNeeded);
        }
        self.token.secret_of(reference)
    }

    fn pin_warning(
        &self,
        reference: &CertificateRef,
    ) -> Result<domain::secret::PinWarning, TokenError> {
        if self.is_installed(reference) {
            return Ok(domain::secret::PinWarning::Quiet);
        }
        self.token.pin_warning(reference)
    }

    fn offers(
        &self,
        reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        self.token.offers(reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        let secret = self.secret_for(reference, secret)?;
        self.token.accepts_the_secret(reference, &secret)
    }

    fn sign_with_secret(
        &self,
        reference: &CertificateRef,
        secret: &ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        let secret = self.secret_for(reference, secret)?;
        self.token
            .sign_with_secret(reference, &secret, algorithm, data)
    }

    fn hold_one_login(&self, reference: &CertificateRef) {
        self.token.hold_one_login(reference);
    }

    fn release_the_login(&self, reference: &CertificateRef) {
        self.token.release_the_login(reference);
    }
}

impl<T: ports::Token + ?Sized> crate::signing::ports::Signer for T {
    fn secret_of(
        &self,
        reference: &domain::certificate::CertificateRef,
    ) -> Result<domain::secret::StoreSecret, domain::error::TokenError> {
        ports::Token::secret_of(self, reference)
    }

    fn pin_warning(
        &self,
        reference: &domain::certificate::CertificateRef,
    ) -> Result<domain::secret::PinWarning, domain::error::TokenError> {
        ports::Token::pin_warning(self, reference)
    }

    fn offers(
        &self,
        reference: &domain::certificate::CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), domain::error::TokenError> {
        ports::Token::offers(self, reference, algorithm)
    }

    fn accepts_the_secret(
        &self,
        reference: &domain::certificate::CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), domain::error::TokenError> {
        ports::Token::accepts_the_secret(self, reference, secret)
    }

    fn sign_with_secret(
        &self,
        reference: &domain::certificate::CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, domain::error::TokenError> {
        ports::Token::sign_with_secret(self, reference, secret, algorithm, data)
    }

    fn hold_one_login(&self, reference: &domain::certificate::CertificateRef) {
        ports::Token::hold_one_login(self, reference);
    }

    fn release_the_login(&self, reference: &domain::certificate::CertificateRef) {
        ports::Token::release_the_login(self, reference);
    }
}

#[cfg(test)]
mod tests;
