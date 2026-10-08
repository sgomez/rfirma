//! Los objetos de la tarjeta, con las etiquetas del DNIe: tres certificados públicos y dos claves privadas.

use cryptoki_sys::{
    CKA_CERTIFICATE_TYPE, CKA_CLASS, CKA_DECRYPT, CKA_EXTRACTABLE, CKA_ID, CKA_ISSUER,
    CKA_KEY_TYPE, CKA_LABEL, CKA_MODIFIABLE, CKA_MODULUS, CKA_PRIVATE, CKA_PRIVATE_EXPONENT,
    CKA_PUBLIC_EXPONENT, CKA_SENSITIVE, CKA_SERIAL_NUMBER, CKA_SIGN, CKA_SUBJECT, CKA_TOKEN,
    CKA_VALUE, CKC_X_509, CKK_RSA, CKO_CERTIFICATE, CKO_PRIVATE_KEY, CK_ATTRIBUTE_TYPE,
    CK_OBJECT_HANDLE, CK_ULONG,
};
use openssl::hash::{hash, MessageDigest};
use openssl::x509::X509;

use crate::material::{Failure, Identity, Material};

/// Un atributo tal como lo pediría quien llama a `C_GetAttributeValue`.
pub(crate) enum Lookup<'a> {
    Missing,
    Sensitive,
    Bytes(&'a [u8]),
}

enum Stored {
    Sensitive,
    Bytes(Vec<u8>),
}

pub(crate) struct Object {
    attributes: Vec<(CK_ATTRIBUTE_TYPE, Stored)>,
    private: bool,
}

impl Object {
    pub(crate) fn is_private(&self) -> bool {
        self.private
    }

    pub(crate) fn lookup(&self, wanted: CK_ATTRIBUTE_TYPE) -> Lookup<'_> {
        match self.attributes.iter().find(|(kind, _)| *kind == wanted) {
            None => Lookup::Missing,
            Some((_, Stored::Sensitive)) => Lookup::Sensitive,
            Some((_, Stored::Bytes(bytes))) => Lookup::Bytes(bytes),
        }
    }

    pub(crate) fn matches(&self, template: &[(CK_ATTRIBUTE_TYPE, Vec<u8>)]) -> bool {
        template.iter().all(
            |(kind, wanted)| matches!(self.lookup(*kind), Lookup::Bytes(bytes) if bytes == wanted),
        )
    }
}

/// Qué identidad de la tarjeta está detrás de una clave privada.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum KeyRole {
    Authentication,
    Signing,
}

pub(crate) const AUTHENTICATION_KEY: CK_OBJECT_HANDLE = 4;
pub(crate) const SIGNING_KEY: CK_OBJECT_HANDLE = 5;

pub(crate) fn key_role(handle: CK_OBJECT_HANDLE) -> Option<KeyRole> {
    match handle {
        AUTHENTICATION_KEY => Some(KeyRole::Authentication),
        SIGNING_KEY => Some(KeyRole::Signing),
        _ => None,
    }
}

/// Los objetos en el orden de sus manejadores, que empiezan en 1.
pub(crate) fn objects_of(material: &Material) -> Result<Vec<Object>, Failure> {
    Ok(vec![
        certificate("CertAutenticacion", &material.authentication.certificate)?,
        certificate("CertFirmaDigital", &material.signing.certificate)?,
        certificate("CertCAIntermediaDGP", &material.intermediate)?,
        private_key("KprivAutenticacion", &material.authentication, true)?,
        private_key("KprivFirmaDigital", &material.signing, false)?,
    ])
}

fn certificate(label: &str, certificate: &X509) -> Result<Object, Failure> {
    Ok(Object {
        attributes: vec![
            (CKA_CLASS, ulong(CKO_CERTIFICATE)),
            (CKA_CERTIFICATE_TYPE, ulong(CKC_X_509)),
            (CKA_TOKEN, boolean(true)),
            (CKA_PRIVATE, boolean(false)),
            (CKA_MODIFIABLE, boolean(false)),
            (CKA_LABEL, bytes(label.as_bytes())),
            (CKA_ID, bytes(&id_of(certificate)?)),
            (CKA_SUBJECT, bytes(&certificate.subject_name().to_der()?)),
            (CKA_ISSUER, bytes(&certificate.issuer_name().to_der()?)),
            (
                CKA_SERIAL_NUMBER,
                bytes(&certificate.serial_number().to_bn()?.to_vec()),
            ),
            (CKA_VALUE, bytes(&certificate.to_der()?)),
        ],
        private: false,
    })
}

fn private_key(label: &str, identity: &Identity, decrypts: bool) -> Result<Object, Failure> {
    let rsa = identity.key.rsa()?;
    Ok(Object {
        attributes: vec![
            (CKA_CLASS, ulong(CKO_PRIVATE_KEY)),
            (CKA_KEY_TYPE, ulong(CKK_RSA)),
            (CKA_TOKEN, boolean(true)),
            (CKA_PRIVATE, boolean(true)),
            (CKA_MODIFIABLE, boolean(false)),
            (CKA_LABEL, bytes(label.as_bytes())),
            (CKA_ID, bytes(&id_of(&identity.certificate)?)),
            (
                CKA_SUBJECT,
                bytes(&identity.certificate.subject_name().to_der()?),
            ),
            (CKA_SIGN, boolean(true)),
            (CKA_DECRYPT, boolean(decrypts)),
            (CKA_SENSITIVE, boolean(true)),
            (CKA_EXTRACTABLE, boolean(false)),
            (CKA_MODULUS, bytes(&rsa.n().to_vec())),
            (CKA_PUBLIC_EXPONENT, bytes(&rsa.e().to_vec())),
            (CKA_PRIVATE_EXPONENT, Stored::Sensitive),
            (CKA_VALUE, Stored::Sensitive),
        ],
        private: true,
    })
}

/// El `CKA_ID` que empareja certificado y clave: el SHA-1 del módulo RSA, como en las tarjetas PKCS#15.
fn id_of(certificate: &X509) -> Result<Vec<u8>, Failure> {
    let modulus = certificate.public_key()?.rsa()?.n().to_vec();
    Ok(hash(MessageDigest::sha1(), &modulus)?.to_vec())
}

fn ulong(value: CK_ULONG) -> Stored {
    Stored::Bytes(value.to_ne_bytes().to_vec())
}

fn boolean(value: bool) -> Stored {
    Stored::Bytes(vec![u8::from(value)])
}

fn bytes(value: &[u8]) -> Stored {
    Stored::Bytes(value.to_vec())
}
