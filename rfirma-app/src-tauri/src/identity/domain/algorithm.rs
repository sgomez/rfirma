//! El algoritmo de firma que se pide por su nombre y el mecanismo PKCS#11 con el que se cumple.

use cryptoki::mechanism::rsa::{PkcsMgfType, PkcsPssParams};
use cryptoki::mechanism::{Mechanism, MechanismType};

/// La clase de clave privada que un algoritmo exige del certificado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyKind {
    /// Clave RSA.
    Rsa,
    /// Clave de curva elíptica.
    Ec,
}

/// Un algoritmo de firma de los que el cliente original acepta por su nombre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// `SHA256withRSA`.
    Sha256Rsa,
    /// `SHA384withRSA`.
    Sha384Rsa,
    /// `SHA512withRSA`.
    Sha512Rsa,
    /// `SHA256withRSAandMGF1`.
    Sha256RsaPss,
    /// `SHA384withRSAandMGF1`.
    Sha384RsaPss,
    /// `SHA512withRSAandMGF1`.
    Sha512RsaPss,
    /// `SHA256withECDSA`.
    Sha256Ecdsa,
    /// `SHA384withECDSA`.
    Sha384Ecdsa,
    /// `SHA512withECDSA`.
    Sha512Ecdsa,
}

impl SignatureAlgorithm {
    /// Todos los algoritmos, en el orden en que los nombra el original.
    pub const ALL: [Self; 9] = [
        Self::Sha256Rsa,
        Self::Sha384Rsa,
        Self::Sha512Rsa,
        Self::Sha256RsaPss,
        Self::Sha384RsaPss,
        Self::Sha512RsaPss,
        Self::Sha256Ecdsa,
        Self::Sha384Ecdsa,
        Self::Sha512Ecdsa,
    ];

    /// El algoritmo que se llama así, sin distinguir mayúsculas.
    pub fn from_name(name: &str) -> Option<Self> {
        let asked = name.trim();
        Self::ALL
            .into_iter()
            .find(|algorithm| algorithm.name().eq_ignore_ascii_case(asked))
    }

    /// El nombre con el que viaja al puente y a la sede.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sha256Rsa => "SHA256withRSA",
            Self::Sha384Rsa => "SHA384withRSA",
            Self::Sha512Rsa => "SHA512withRSA",
            Self::Sha256RsaPss => "SHA256withRSAandMGF1",
            Self::Sha384RsaPss => "SHA384withRSAandMGF1",
            Self::Sha512RsaPss => "SHA512withRSAandMGF1",
            Self::Sha256Ecdsa => "SHA256withECDSA",
            Self::Sha384Ecdsa => "SHA384withECDSA",
            Self::Sha512Ecdsa => "SHA512withECDSA",
        }
    }

    /// La clase de clave privada con la que se puede usar.
    pub fn key_kind(self) -> KeyKind {
        match self {
            Self::Sha256Ecdsa | Self::Sha384Ecdsa | Self::Sha512Ecdsa => KeyKind::Ec,
            _ => KeyKind::Rsa,
        }
    }

    /// El mecanismo que el token tiene que ofrecer para cumplirlo.
    pub fn mechanism_type(self) -> MechanismType {
        match self {
            Self::Sha256Rsa => MechanismType::SHA256_RSA_PKCS,
            Self::Sha384Rsa => MechanismType::SHA384_RSA_PKCS,
            Self::Sha512Rsa => MechanismType::SHA512_RSA_PKCS,
            Self::Sha256RsaPss => MechanismType::SHA256_RSA_PKCS_PSS,
            Self::Sha384RsaPss => MechanismType::SHA384_RSA_PKCS_PSS,
            Self::Sha512RsaPss => MechanismType::SHA512_RSA_PKCS_PSS,
            Self::Sha256Ecdsa => MechanismType::ECDSA_SHA256,
            Self::Sha384Ecdsa => MechanismType::ECDSA_SHA384,
            Self::Sha512Ecdsa => MechanismType::ECDSA_SHA512,
        }
    }

    /// El mecanismo compuesto, sobre los bytes sin hashear, con el que se invoca `C_Sign`.
    pub fn mechanism(self) -> Mechanism<'static> {
        match self {
            Self::Sha256Rsa => Mechanism::Sha256RsaPkcs,
            Self::Sha384Rsa => Mechanism::Sha384RsaPkcs,
            Self::Sha512Rsa => Mechanism::Sha512RsaPkcs,
            Self::Sha256RsaPss => Mechanism::Sha256RsaPkcsPss(pss(
                MechanismType::SHA256,
                PkcsMgfType::MGF1_SHA256,
                32,
            )),
            Self::Sha384RsaPss => Mechanism::Sha384RsaPkcsPss(pss(
                MechanismType::SHA384,
                PkcsMgfType::MGF1_SHA384,
                48,
            )),
            Self::Sha512RsaPss => Mechanism::Sha512RsaPkcsPss(pss(
                MechanismType::SHA512,
                PkcsMgfType::MGF1_SHA512,
                64,
            )),
            Self::Sha256Ecdsa => Mechanism::EcdsaSha256,
            Self::Sha384Ecdsa => Mechanism::EcdsaSha384,
            Self::Sha512Ecdsa => Mechanism::EcdsaSha512,
        }
    }
}

fn pss(hash_alg: MechanismType, mgf: PkcsMgfType, salt_length: u64) -> PkcsPssParams {
    PkcsPssParams {
        hash_alg,
        mgf,
        s_len: salt_length.into(),
    }
}

#[cfg(test)]
mod tests;
