//! El token de grada A de las pruebas del trámite: firma lo que se le pida con los mecanismos que declara, o se bloquea tras unas firmas.

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::secret::StoreSecret;
use crate::signing::ports::Signer;

/// Un token que firma cualquier cosa con los mecanismos que declara, sin PKCS#11 delante.
pub(crate) struct ATokenThatSigns {
    offered: Vec<SignatureAlgorithm>,
    secrets_asked: std::sync::Mutex<usize>,
    signed_with: std::sync::Mutex<Vec<SignatureAlgorithm>>,
    locked_after: Option<usize>,
    attempts: std::sync::Mutex<usize>,
}

impl Default for ATokenThatSigns {
    fn default() -> Self {
        Self::offering(&SignatureAlgorithm::ALL)
    }
}

impl ATokenThatSigns {
    pub(crate) fn offering(offered: &[SignatureAlgorithm]) -> Self {
        Self {
            offered: offered.to_vec(),
            secrets_asked: std::sync::Mutex::new(0),
            signed_with: std::sync::Mutex::new(Vec::new()),
            locked_after: None,
            attempts: std::sync::Mutex::new(0),
        }
    }

    /// Una tarjeta que firma `signatures` veces y luego se bloquea: las siguientes firmas fallan con el PIN bloqueado.
    pub(crate) fn locked_after(signatures: usize) -> Self {
        Self {
            locked_after: Some(signatures),
            ..Self::default()
        }
    }

    pub(crate) fn secrets_asked(&self) -> usize {
        *crate::lock(&self.secrets_asked)
    }

    /// Cuántas firmas le han llegado, salieran o no.
    pub(crate) fn attempts(&self) -> usize {
        *crate::lock(&self.attempts)
    }

    /// Los algoritmos con los que firmó, en orden.
    pub(crate) fn signed_with(&self) -> Vec<SignatureAlgorithm> {
        crate::lock(&self.signed_with).clone()
    }
}

impl Signer for ATokenThatSigns {
    fn accepts_the_secret(
        &self,
        _reference: &crate::identity::domain::certificate::CertificateRef,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), crate::identity::domain::error::TokenError> {
        Ok(())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        *crate::lock(&self.secrets_asked) += 1;
        Ok(StoreSecret::NotNeeded)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        if self.offered.contains(&algorithm) {
            return Ok(());
        }
        Err(TokenError::new(
            Situation::MechanismNotOffered,
            format!(
                "el token no firma {} con {}: no esta entre los mecanismos de la ranura",
                algorithm.name(),
                algorithm.mechanism_type()
            ),
        ))
    }

    fn sign_with_secret(
        &self,
        _reference: &CertificateRef,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        *crate::lock(&self.attempts) += 1;
        let mut signed_with = crate::lock(&self.signed_with);
        if self
            .locked_after
            .is_some_and(|limit| signed_with.len() >= limit)
        {
            return Err(TokenError::new(
                Situation::PinLocked,
                "la tarjeta se ha bloqueado",
            ));
        }
        signed_with.push(algorithm);
        Ok(vec![0x01; 256])
    }
}
