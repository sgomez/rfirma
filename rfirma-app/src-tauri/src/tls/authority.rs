//! Autoridad de certificación (CA) local para firmar el certificado del servidor (ADR-0005).

use openssl::asn1::{Asn1Integer, Asn1Object, Asn1OctetString, Asn1Time};
use openssl::bn::{BigNum, MsbOption};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Extension, X509Name, X509};

use super::error::{Situation, TlsError};

/// Días de validez de la CA local (ADR-0005).
pub const VALIDITY_DAYS: u32 = 900;

/// Nombre común (CN) de la CA local.
pub const COMMON_NAME: &str = "rFirma CA local";

/// Nombre DNS permitido por la restricción de nombres.
pub const PERMITTED_DNS_NAME: &str = "localhost";
/// Dirección IPv4 de loopback permitida por la restricción de nombres.
pub const PERMITTED_IPV4: [u8; 4] = [127, 0, 0, 1];
/// Dirección IPv6 de loopback permitida por la restricción de nombres.
pub const PERMITTED_IPV6: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

/// Autoridad de certificación local con su certificado y clave privada.
#[derive(Clone)]
pub struct LocalCa {
    certificate: X509,
    key: PKey<Private>,
}

impl LocalCa {
    /// Genera una CA local nueva válida desde este momento.
    pub fn generate() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, VALIDITY_DAYS).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// Genera una CA local a punto de caducar para pruebas.
    #[cfg(test)]
    pub fn almost_expired_for_test() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, 2).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// Genera una CA local ya caducada para pruebas.
    #[cfg(test)]
    pub fn expired_for_test() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, 0).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// Reconstruye la CA local a partir de los PEM de certificado y clave privada.
    pub fn from_pem(certificate_pem: &[u8], key_pem: &[u8]) -> Result<Self, TlsError> {
        let certificate = X509::from_pem(certificate_pem).map_err(damaged)?;
        let key = PKey::private_key_from_pem(key_pem).map_err(damaged)?;
        let public = certificate.public_key().map_err(damaged)?;
        if !public.public_eq(&key) {
            return Err(TlsError::new(
                Situation::MaterialDamaged,
                "la clave guardada no es la del certificado de la CA local",
            ));
        }
        Ok(Self { certificate, key })
    }

    /// Certificado de la CA local.
    pub fn certificate(&self) -> &X509 {
        &self.certificate
    }

    /// Clave privada de la CA local.
    pub fn key(&self) -> &PKey<Private> {
        &self.key
    }

    /// Certificado codificado en formato PEM.
    pub fn certificate_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.certificate.to_pem().map_err(not_generated)
    }

    /// Clave privada codificada en formato PEM PKCS#8 sin cifrar (ADR-0005).
    pub fn private_key_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.key.private_key_to_pem_pkcs8().map_err(not_generated)
    }

    /// Días restantes de validez de la CA local.
    pub fn days_left(&self) -> Result<i64, TlsError> {
        let now = Asn1Time::days_from_now(0).map_err(damaged)?;
        let difference = now.diff(self.certificate.not_after()).map_err(damaged)?;
        Ok(i64::from(difference.days))
    }
}

impl std::fmt::Debug for LocalCa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCa")
            .field("not_after", &self.certificate.not_after().to_string())
            .finish_non_exhaustive()
    }
}

pub(super) fn generate_key() -> Result<PKey<Private>, TlsError> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(not_generated)?;
    let key = EcKey::generate(&group).map_err(not_generated)?;
    PKey::from_ec_key(key).map_err(not_generated)
}

pub(super) fn random_serial() -> Result<Asn1Integer, openssl::error::ErrorStack> {
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    serial.to_asn1_integer()
}

fn build_certificate(
    key: &PKey<Private>,
    validity_days: u32,
) -> Result<X509, openssl::error::ErrorStack> {
    let mut name = X509Name::builder()?;
    name.append_entry_by_nid(Nid::COMMONNAME, COMMON_NAME)?;
    let name = name.build();

    let mut builder = X509::builder()?;
    builder.set_version(2)?;
    let serial = random_serial()?;
    builder.set_serial_number(&serial)?;
    builder.set_subject_name(&name)?;
    builder.set_issuer_name(&name)?;
    builder.set_pubkey(key)?;
    let not_before = Asn1Time::days_from_now(0)?;
    let not_after = Asn1Time::days_from_now(validity_days)?;
    builder.set_not_before(&not_before)?;
    builder.set_not_after(&not_after)?;
    builder.append_extension(BasicConstraints::new().critical().ca().pathlen(0).build()?)?;
    builder.append_extension(
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()?,
    )?;
    builder.append_extension(name_constraints()?)?;
    let identifier = {
        let context = builder.x509v3_context(None, None);
        SubjectKeyIdentifier::new().build(&context)?
    };
    builder.append_extension(identifier)?;
    builder.sign(key, MessageDigest::sha256())?;
    Ok(builder.build())
}

fn name_constraints() -> Result<X509Extension, openssl::error::ErrorStack> {
    const DNS_NAME: u8 = 0x82;
    const IP_ADDRESS: u8 = 0x87;
    const SEQUENCE: u8 = 0x30;
    const PERMITTED_SUBTREES: u8 = 0xa0;

    let mut ipv4 = PERMITTED_IPV4.to_vec();
    ipv4.extend_from_slice(&[0xff; 4]);
    let mut ipv6 = PERMITTED_IPV6.to_vec();
    ipv6.extend_from_slice(&[0xff; 16]);

    let mut subtrees = Vec::new();
    for base in [
        tagged(DNS_NAME, PERMITTED_DNS_NAME.as_bytes()),
        tagged(IP_ADDRESS, &ipv4),
        tagged(IP_ADDRESS, &ipv6),
    ] {
        subtrees.extend_from_slice(&tagged(SEQUENCE, &base));
    }
    let permitted = tagged(PERMITTED_SUBTREES, &subtrees);
    let der = tagged(SEQUENCE, &permitted);

    let oid = Asn1Object::from_str("2.5.29.30")?;
    let contents = Asn1OctetString::new_from_bytes(&der)?;
    X509Extension::new_from_der(&oid, true, &contents)
}

fn tagged(tag: u8, contents: &[u8]) -> Vec<u8> {
    assert!(
        contents.len() < 0x80,
        "la longitud en forma corta llega hasta 127 bytes"
    );
    let mut out = Vec::with_capacity(contents.len() + 2);
    out.push(tag);
    out.push(contents.len() as u8);
    out.extend_from_slice(contents);
    out
}

fn not_generated(error: openssl::error::ErrorStack) -> TlsError {
    TlsError::new(Situation::MaterialNotGenerated, error.to_string())
}

fn damaged(error: openssl::error::ErrorStack) -> TlsError {
    TlsError::new(Situation::MaterialDamaged, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(ca: &LocalCa) -> String {
        String::from_utf8(ca.certificate().to_text().expect("deberia imprimirse"))
            .expect("deberia ser UTF-8")
    }

    #[test]
    fn the_local_ca_can_only_vouch_for_the_loopback() {
        let text = text_of(&LocalCa::generate().expect("deberia generarse"));

        assert!(
            text.contains("X509v3 Name Constraints: critical"),
            "sin la restriccion la CA local podria afirmar cualquier sitio web:\n{text}"
        );
        assert!(text.contains("DNS:localhost"), "{text}");
        assert!(text.contains("IP:127.0.0.1/255.255.255.255"), "{text}");
        assert!(
            text.contains("IP:0:0:0:0:0:0:0:1/FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF"),
            "{text}"
        );
    }

    #[test]
    fn the_local_ca_signs_certificates_and_nothing_else() {
        let text = text_of(&LocalCa::generate().expect("deberia generarse"));

        assert!(text.contains("CA:TRUE, pathlen:0"), "{text}");
        assert!(
            text.contains("Certificate Sign, CRL Sign"),
            "el keyUsage se reduce a firmar certificados (ADR-0005):\n{text}"
        );
        assert!(
            !text.contains("Digital Signature"),
            "una CA local que sirva ademas para TLS es otra cosa:\n{text}"
        );
    }

    #[test]
    fn the_local_ca_expires_between_two_and_three_years_from_now() {
        let ca = LocalCa::generate().expect("deberia generarse");

        let two_years = Asn1Time::days_from_now(2 * 365).expect("deberia calcularse");
        let three_years = Asn1Time::days_from_now(3 * 365).expect("deberia calcularse");
        assert!(
            ca.certificate().not_after() > two_years.as_ref(),
            "la caducidad debe superar los dos años"
        );
        assert!(
            ca.certificate().not_after() < three_years.as_ref(),
            "la caducidad debe ser menor a tres años"
        );
    }

    #[test]
    fn two_local_ca_are_never_the_same_certificate() {
        let one = LocalCa::generate().expect("deberia generarse");
        let another = LocalCa::generate().expect("deberia generarse");

        assert_ne!(
            one.certificate().serial_number().to_bn().unwrap().to_vec(),
            another
                .certificate()
                .serial_number()
                .to_bn()
                .unwrap()
                .to_vec(),
            "el solape convive con dos CA locales vivas, y se distinguen por el serie"
        );
    }

    #[test]
    fn a_local_ca_survives_the_round_trip_through_the_two_pem_files() {
        let original = LocalCa::generate().expect("deberia generarse");

        let restored = LocalCa::from_pem(
            &original.certificate_pem().expect("deberia salir en PEM"),
            &original.private_key_pem().expect("deberia salir en PEM"),
        )
        .expect("deberia releerse");

        assert_eq!(
            restored.certificate().to_pem().unwrap(),
            original.certificate().to_pem().unwrap()
        );
    }

    #[test]
    fn a_certificate_with_someone_elses_key_is_damaged_material() {
        let one = LocalCa::generate().expect("deberia generarse");
        let another = LocalCa::generate().expect("deberia generarse");

        let error = LocalCa::from_pem(
            &one.certificate_pem().expect("deberia salir en PEM"),
            &another.private_key_pem().expect("deberia salir en PEM"),
        )
        .expect_err("un par que no se corresponde no es una CA local");

        assert_eq!(error.situation(), Situation::MaterialDamaged);
    }

    #[test]
    fn the_stored_private_key_is_plain_pkcs8_and_not_the_certificate() {
        let ca = LocalCa::generate().expect("deberia generarse");

        let key = String::from_utf8(ca.private_key_pem().expect("deberia salir en PEM"))
            .expect("deberia ser UTF-8");

        assert!(key.starts_with("-----BEGIN PRIVATE KEY-----"), "{key}");
        assert!(!key.contains("ENCRYPTED"), "la clave se guarda sin cifrar");
    }

    #[test]
    fn the_debug_output_never_carries_the_private_key() {
        let ca = LocalCa::generate().expect("deberia generarse");

        let printed = format!("{ca:?}");

        assert!(printed.contains("LocalCa"), "{printed}");
        assert!(!printed.contains("PRIVATE"), "{printed}");
    }
}
