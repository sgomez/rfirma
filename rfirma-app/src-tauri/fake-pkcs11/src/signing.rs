//! La firma RSA PKCS#1 v1.5 de verdad con las claves de la tarjeta, cruda o con su resumen.

use cryptoki_sys::{
    CKM_RSA_PKCS, CKM_SHA1_RSA_PKCS, CKM_SHA256_RSA_PKCS, CKM_SHA384_RSA_PKCS, CKM_SHA512_RSA_PKCS,
    CKR_DATA_LEN_RANGE, CKR_FUNCTION_FAILED, CK_MECHANISM_TYPE, CK_RV,
};
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Padding;
use openssl::sign::Signer;

use crate::material::Material;
use crate::objects::KeyRole;

/// Los mecanismos de firma que ofrece la ranura.
pub(crate) const MECHANISMS: [CK_MECHANISM_TYPE; 5] = [
    CKM_RSA_PKCS,
    CKM_SHA1_RSA_PKCS,
    CKM_SHA256_RSA_PKCS,
    CKM_SHA384_RSA_PKCS,
    CKM_SHA512_RSA_PKCS,
];

pub(crate) fn signature_len(material: &Material, role: KeyRole) -> usize {
    key_of(material, role).size()
}

pub(crate) fn sign(
    material: &Material,
    role: KeyRole,
    mechanism: CK_MECHANISM_TYPE,
    data: &[u8],
) -> Result<Vec<u8>, CK_RV> {
    let key = key_of(material, role);
    match digest_of(mechanism) {
        Some(digest) => sign_digesting(key, digest, data),
        None => sign_raw(key, data),
    }
}

fn key_of(material: &Material, role: KeyRole) -> &PKey<Private> {
    match role {
        KeyRole::Authentication => &material.authentication.key,
        KeyRole::Signing => &material.signing.key,
    }
}

fn digest_of(mechanism: CK_MECHANISM_TYPE) -> Option<MessageDigest> {
    match mechanism {
        CKM_SHA1_RSA_PKCS => Some(MessageDigest::sha1()),
        CKM_SHA256_RSA_PKCS => Some(MessageDigest::sha256()),
        CKM_SHA384_RSA_PKCS => Some(MessageDigest::sha384()),
        CKM_SHA512_RSA_PKCS => Some(MessageDigest::sha512()),
        _ => None,
    }
}

fn sign_digesting(
    key: &PKey<Private>,
    digest: MessageDigest,
    data: &[u8],
) -> Result<Vec<u8>, CK_RV> {
    let mut signer = Signer::new(digest, key).map_err(|_| CKR_FUNCTION_FAILED)?;
    signer.update(data).map_err(|_| CKR_FUNCTION_FAILED)?;
    signer.sign_to_vec().map_err(|_| CKR_FUNCTION_FAILED)
}

/// `CKM_RSA_PKCS`: rellena y cifra con la clave privada lo que llega, que ya es un `DigestInfo`.
fn sign_raw(key: &PKey<Private>, data: &[u8]) -> Result<Vec<u8>, CK_RV> {
    let rsa = key.rsa().map_err(|_| CKR_FUNCTION_FAILED)?;
    let mut signature = vec![0; rsa.size() as usize];
    let written = rsa
        .private_encrypt(data, &mut signature, Padding::PKCS1)
        .map_err(|_| CKR_DATA_LEN_RANGE)?;
    signature.truncate(written);
    Ok(signature)
}
