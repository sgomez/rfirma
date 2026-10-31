//! El `DigestInfo` de PKCS#1 v1.5 que firma el mecanismo RSA crudo cuando la ranura no ofrece el compuesto.

use cryptoki::mechanism::MechanismType;

use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::digest::digest;
use crate::identity::domain::error::{Situation, TokenError};

/// El mecanismo que firma un `DigestInfo` ya armado, sin resumir ni envolver nada.
pub const OVER_A_DIGEST_INFO: MechanismType = MechanismType::RSA_PKCS;

const SHA1_PREFIX: &[u8] = &[
    0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
];
const SHA256_PREFIX: &[u8] = &[
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];
const SHA384_PREFIX: &[u8] = &[
    0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02, 0x05,
    0x00, 0x04, 0x30,
];
const SHA512_PREFIX: &[u8] = &[
    0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03, 0x05,
    0x00, 0x04, 0x40,
];

/// Si el algoritmo es RSA PKCS#1 v1.5, el único que se cumple firmando un `DigestInfo`.
pub fn signs_a_digest_info(algorithm: SignatureAlgorithm) -> bool {
    prefix_of(algorithm).is_some()
}

/// El `DigestInfo` DER del resumen de `data` (RFC 8017, §9.2).
pub fn digest_info(algorithm: SignatureAlgorithm, data: &[u8]) -> Result<Vec<u8>, TokenError> {
    let prefix = prefix_of(algorithm).ok_or_else(|| {
        TokenError::new(
            Situation::Unknown,
            format!("{} no se firma sobre un DigestInfo", algorithm.name()),
        )
    })?;
    Ok([prefix, &digest(algorithm, data)?].concat())
}

fn prefix_of(algorithm: SignatureAlgorithm) -> Option<&'static [u8]> {
    match algorithm {
        SignatureAlgorithm::Sha1Rsa => Some(SHA1_PREFIX),
        SignatureAlgorithm::Sha256Rsa => Some(SHA256_PREFIX),
        SignatureAlgorithm::Sha384Rsa => Some(SHA384_PREFIX),
        SignatureAlgorithm::Sha512Rsa => Some(SHA512_PREFIX),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
