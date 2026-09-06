//! Contexto `identity` (ADR-0017).

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

impl<T: ports::Token + ?Sized> crate::signing::ports::Signer for T {
    fn secret_of(
        &self,
        reference: &domain::certificate::CertificateRef,
    ) -> Result<domain::secret::StoreSecret, domain::error::TokenError> {
        ports::Token::secret_of(self, reference)
    }

    fn sign(
        &self,
        reference: &domain::certificate::CertificateRef,
        pin: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, domain::error::TokenError> {
        ports::Token::sign(self, reference, pin, data)
    }
}
