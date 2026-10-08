//! Las claves y los certificados de la tarjeta: se generan la primera vez y se guardan en su directorio.

use std::error::Error;
use std::fs;
use std::path::Path;

use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Builder, X509Name, X509NameBuilder, X509};

const KEY_BITS: u32 = 2048;
const VALIDITY_DAYS: u32 = 5 * 365;

/// Una clave privada de la tarjeta y su certificado.
pub(crate) struct Identity {
    pub(crate) key: PKey<Private>,
    pub(crate) certificate: X509,
}

/// Lo que lleva la tarjeta: dos identidades y la CA intermedia que las emite, sin su clave.
pub(crate) struct Material {
    pub(crate) authentication: Identity,
    pub(crate) signing: Identity,
    pub(crate) intermediate: X509,
}

pub(crate) type Failure = Box<dyn Error + Send + Sync>;

pub(crate) fn load_or_generate(dir: &Path) -> Result<Material, Failure> {
    if dir.join("intermediate.der").is_file() {
        return load(dir);
    }
    let material = generate()?;
    save(dir, &material)?;
    Ok(material)
}

fn load(dir: &Path) -> Result<Material, Failure> {
    Ok(Material {
        authentication: load_identity(dir, "authentication")?,
        signing: load_identity(dir, "signing")?,
        intermediate: X509::from_der(&fs::read(dir.join("intermediate.der"))?)?,
    })
}

fn load_identity(dir: &Path, name: &str) -> Result<Identity, Failure> {
    Ok(Identity {
        key: PKey::private_key_from_der(&fs::read(dir.join(format!("{name}.key.der")))?)?,
        certificate: X509::from_der(&fs::read(dir.join(format!("{name}.der")))?)?,
    })
}

fn save(dir: &Path, material: &Material) -> Result<(), Failure> {
    save_identity(dir, "authentication", &material.authentication)?;
    save_identity(dir, "signing", &material.signing)?;
    fs::write(
        dir.join("intermediate.der"),
        material.intermediate.to_der()?,
    )?;
    Ok(())
}

fn save_identity(dir: &Path, name: &str, identity: &Identity) -> Result<(), Failure> {
    fs::write(
        dir.join(format!("{name}.key.der")),
        identity.key.private_key_to_der()?,
    )?;
    fs::write(
        dir.join(format!("{name}.der")),
        identity.certificate.to_der()?,
    )?;
    Ok(())
}

fn generate() -> Result<Material, Failure> {
    let root_key = rsa_key()?;
    let root_name = authority_name("AC RAIZ FALSA DE PRUEBAS")?;
    let intermediate_key = rsa_key()?;
    let intermediate_name = authority_name("AC DNIE 004")?;
    let intermediate =
        authority_certificate(&intermediate_name, &intermediate_key, &root_name, &root_key)?;

    Ok(Material {
        authentication: holder_identity(
            "AUTENTICACIÓN",
            KeyUsage::new().critical().digital_signature().build()?,
            &intermediate,
            &intermediate_key,
        )?,
        signing: holder_identity(
            "FIRMA",
            KeyUsage::new().critical().non_repudiation().build()?,
            &intermediate,
            &intermediate_key,
        )?,
        intermediate,
    })
}

fn rsa_key() -> Result<PKey<Private>, Failure> {
    Ok(PKey::from_rsa(Rsa::generate(KEY_BITS)?)?)
}

fn authority_name(common_name: &str) -> Result<X509Name, Failure> {
    let mut name = X509NameBuilder::new()?;
    name.append_entry_by_text("C", "ES")?;
    name.append_entry_by_text("O", "DIRECCION GENERAL DE LA POLICIA")?;
    name.append_entry_by_text("OU", "DNIE")?;
    name.append_entry_by_text("CN", common_name)?;
    Ok(name.build())
}

fn holder_name(purpose: &str) -> Result<X509Name, Failure> {
    let mut name = X509NameBuilder::new()?;
    name.append_entry_by_text("C", "ES")?;
    name.append_entry_by_text("serialNumber", "00000000T")?;
    name.append_entry_by_text("SN", "PRUEBA")?;
    name.append_entry_by_text("GN", "TARJETA FALSA")?;
    name.append_entry_by_text("CN", &format!("PRUEBA, TARJETA FALSA ({purpose})"))?;
    Ok(name.build())
}

fn authority_certificate(
    subject: &X509Name,
    key: &PKey<Private>,
    issuer: &X509Name,
    issuer_key: &PKey<Private>,
) -> Result<X509, Failure> {
    let mut builder = certificate_builder(subject, key, issuer)?;
    builder.append_extension(BasicConstraints::new().critical().ca().pathlen(0).build()?)?;
    builder.append_extension(
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()?,
    )?;
    builder.sign(issuer_key, MessageDigest::sha256())?;
    Ok(builder.build())
}

fn holder_identity(
    purpose: &str,
    usage: openssl::x509::X509Extension,
    issuer: &X509,
    issuer_key: &PKey<Private>,
) -> Result<Identity, Failure> {
    let key = rsa_key()?;
    let subject = holder_name(purpose)?;
    let mut builder = certificate_builder(&subject, &key, issuer.subject_name())?;
    builder.append_extension(BasicConstraints::new().build()?)?;
    builder.append_extension(usage)?;
    builder.sign(issuer_key, MessageDigest::sha256())?;
    Ok(Identity {
        key,
        certificate: builder.build(),
    })
}

fn certificate_builder(
    subject: &openssl::x509::X509NameRef,
    key: &PKey<Private>,
    issuer: &openssl::x509::X509NameRef,
) -> Result<X509Builder, Failure> {
    let mut builder = X509Builder::new()?;
    builder.set_version(2)?;
    let mut serial = BigNum::new()?;
    serial.rand(64, MsbOption::MAYBE_ZERO, false)?;
    let serial = serial.to_asn1_integer()?;
    builder.set_serial_number(&serial)?;
    builder.set_subject_name(subject)?;
    builder.set_issuer_name(issuer)?;
    builder.set_pubkey(key)?;
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(VALIDITY_DAYS)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    let identifier = SubjectKeyIdentifier::new().build(&builder.x509v3_context(None, None))?;
    builder.append_extension(identifier)?;
    Ok(builder)
}
