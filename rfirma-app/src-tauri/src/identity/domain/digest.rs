//! El resumen que nombra un algoritmo de firma, el que firman los mecanismos que no resumen por sí mismos.

use openssl::hash::{hash, MessageDigest};

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::error::{Situation, TokenError};

/// El resumen de `data` con la función que nombra el algoritmo.
pub fn digest(algorithm: SignatureAlgorithm, data: &[u8]) -> Result<Vec<u8>, TokenError> {
    hash(named_by(algorithm), data)
        .map(|digest| digest.to_vec())
        .map_err(|error| {
            TokenError::new(
                Situation::Unknown,
                format!("no se ha podido resumir lo que se iba a firmar: {error}"),
            )
        })
}

fn named_by(algorithm: SignatureAlgorithm) -> MessageDigest {
    match algorithm {
        SignatureAlgorithm::Sha1Rsa | SignatureAlgorithm::Sha1Ecdsa => MessageDigest::sha1(),
        SignatureAlgorithm::Sha256Rsa
        | SignatureAlgorithm::Sha256RsaPss
        | SignatureAlgorithm::Sha256Ecdsa => MessageDigest::sha256(),
        SignatureAlgorithm::Sha384Rsa
        | SignatureAlgorithm::Sha384RsaPss
        | SignatureAlgorithm::Sha384Ecdsa => MessageDigest::sha384(),
        SignatureAlgorithm::Sha512Rsa
        | SignatureAlgorithm::Sha512RsaPss
        | SignatureAlgorithm::Sha512Ecdsa => MessageDigest::sha512(),
    }
}

#[cfg(test)]
mod tests;
