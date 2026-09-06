//! Certificado efímero del servidor local emitido por la CA local (ADR-0005).

use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::extension::{
    AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName,
    SubjectKeyIdentifier,
};
use openssl::x509::{X509Name, X509};

use super::authority::{generate_key, random_serial, LocalCa, PERMITTED_DNS_NAME};
use super::error::{Situation, TlsError};

/// Nombre común (CN) del certificado del servidor local.
pub const COMMON_NAME: &str = PERMITTED_DNS_NAME;

/// Días de validez del certificado del servidor local.
pub const VALIDITY_DAYS: u32 = 30;

/// Certificado efímero del servidor local y su clave privada en memoria.
#[derive(Clone)]
pub struct LocalServerCertificate {
    certificate: X509,
    key: PKey<Private>,
}

impl LocalServerCertificate {
    /// Emite el certificado del servidor local firmado por la CA local.
    pub fn issued_by(ca: &LocalCa) -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = issue(ca, &key, COMMON_NAME, |names| {
            names.dns(PERMITTED_DNS_NAME).ip("127.0.0.1");
        })
        .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))?;
        Ok(Self { certificate, key })
    }

    /// Certificado del servidor local.
    pub fn certificate(&self) -> &X509 {
        &self.certificate
    }

    /// Clave privada del servidor local en memoria.
    pub fn key(&self) -> &PKey<Private> {
        &self.key
    }

    /// Certificado codificado en formato PEM.
    pub fn certificate_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.certificate
            .to_pem()
            .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))
    }

    /// Clave privada codificada en formato PEM PKCS#8.
    pub fn private_key_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.key
            .private_key_to_pem_pkcs8()
            .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))
    }
}

impl std::fmt::Debug for LocalServerCertificate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalServerCertificate")
            .field("not_after", &self.certificate.not_after().to_string())
            .finish_non_exhaustive()
    }
}

fn issue(
    ca: &LocalCa,
    key: &PKey<Private>,
    common_name: &str,
    alternative_names: impl Fn(&mut SubjectAlternativeName),
) -> Result<X509, openssl::error::ErrorStack> {
    let mut name = X509Name::builder()?;
    name.append_entry_by_nid(Nid::COMMONNAME, common_name)?;
    let name = name.build();

    let mut builder = X509::builder()?;
    builder.set_version(2)?;
    let serial = random_serial()?;
    builder.set_serial_number(&serial)?;
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(ca.certificate().subject_name())?;
    builder.set_pubkey(key)?;
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(VALIDITY_DAYS)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    builder.append_extension(BasicConstraints::new().critical().build()?)?;
    builder.append_extension(
        KeyUsage::new()
            .critical()
            .digital_signature()
            .key_encipherment()
            .build()?,
    )?;
    builder.append_extension(ExtendedKeyUsage::new().server_auth().build()?)?;

    let mut names = SubjectAlternativeName::new();
    alternative_names(&mut names);
    let (subject_alternative_name, subject_identifier, authority_identifier) = {
        let context = builder.x509v3_context(Some(ca.certificate()), None);
        (
            names.build(&context)?,
            SubjectKeyIdentifier::new().build(&context)?,
            AuthorityKeyIdentifier::new().keyid(false).build(&context)?,
        )
    };
    builder.append_extension(subject_alternative_name)?;
    builder.append_extension(subject_identifier)?;
    builder.append_extension(authority_identifier)?;

    builder.sign(ca.key(), MessageDigest::sha256())?;
    Ok(builder.build())
}

#[cfg(test)]
mod tests;
