//! La CA local, pura, que firma el certificado del servidor: la genera con la marca de su canal y la lee de PEM sin tocar el disco (ADR-0005).

use openssl::asn1::{Asn1Integer, Asn1Object, Asn1OctetString, Asn1Time};
use openssl::bn::{BigNum, MsbOption};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Extension, X509Name, X509};
use x509_cert::der::asn1::{AnyRef, Utf8StringRef};
use x509_cert::der::{Decode, Reader, SliceReader, Tag, TagNumber, Tagged};

use crate::site::domain::tls_error::{Situation, TlsError};

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

/// OID propio de la extensión que marca el canal de la CA local, bajo el arco de UUID (ADR-0005).
pub const CHANNEL_MARK_OID: &str = "2.25.204984766305632451904566026227780766075";

const EXTENSIONS: Tag = Tag::ContextSpecific {
    constructed: true,
    number: TagNumber(3),
};

/// El canal cuya marca lleva dentro una CA local (ADR-0005).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelMark {
    /// El deb y el rpm, que comparten carpeta.
    Native,
    /// El flatpak.
    Flatpak,
    /// Windows.
    Windows,
}

impl ChannelMark {
    const ALL: [Self; 3] = [Self::Native, Self::Flatpak, Self::Windows];

    /// El valor que la extensión guarda para este canal.
    pub fn value(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Flatpak => "flatpak",
            Self::Windows => "windows",
        }
    }

    /// La marca que lleva un certificado en DER, o ninguna si no la lleva o no es un certificado.
    pub fn of_certificate(certificate_der: &[u8]) -> Option<Self> {
        let oid = Asn1Object::from_str(CHANNEL_MARK_OID).ok()?;
        let certificate = AnyRef::from_der(certificate_der).ok()?;
        let tbs = *elements_of(certificate)?.first()?;
        let extensions = elements_of(tbs)?
            .into_iter()
            .find(|field| field.tag() == EXTENSIONS)?;
        let list = *elements_of(extensions)?.first()?;
        let value = elements_of(list)?.into_iter().find_map(|extension| {
            let parts = elements_of(extension)?;
            let id = parts.first()?;
            (id.tag() == Tag::ObjectIdentifier && id.value() == oid.as_slice())
                .then(|| parts.last().copied())
                .flatten()
        })?;
        let text = Utf8StringRef::from_der(value.value()).ok()?;
        Self::ALL
            .into_iter()
            .find(|mark| mark.value() == text.as_str())
    }

    /// Si el canal sustituye una CA local vigente sin marca al instalar: solo los de NSS (ADR-0005).
    pub fn replaces_an_unmarked_local_ca(self) -> bool {
        self != Self::Windows
    }
}

fn elements_of(constructed: AnyRef<'_>) -> Option<Vec<AnyRef<'_>>> {
    let mut reader = SliceReader::new(constructed.value()).ok()?;
    let mut found = Vec::new();
    while !reader.is_finished() {
        found.push(AnyRef::decode(&mut reader).ok()?);
    }
    Some(found)
}

/// Autoridad de certificación local con su certificado y clave privada.
#[derive(Clone)]
pub struct LocalCa {
    certificate: X509,
    key: PKey<Private>,
}

impl LocalCa {
    /// Genera una CA local nueva, válida desde este momento y con la marca de su canal.
    pub fn generate(mark: ChannelMark) -> Result<Self, TlsError> {
        Self::made(VALIDITY_DAYS, Some(mark))
    }

    /// Genera una CA local sin marca de canal, como las de antes de ella, para pruebas.
    #[cfg(test)]
    pub fn unmarked_for_test() -> Result<Self, TlsError> {
        Self::made(VALIDITY_DAYS, None)
    }

    /// Genera una CA local a punto de caducar para pruebas.
    #[cfg(test)]
    pub fn almost_expired_for_test() -> Result<Self, TlsError> {
        Self::made(2, Some(ChannelMark::Native))
    }

    /// Genera una CA local ya caducada para pruebas.
    #[cfg(test)]
    pub fn expired_for_test() -> Result<Self, TlsError> {
        Self::made(0, Some(ChannelMark::Native))
    }

    /// Genera una CA local con los días de validez indicados, para pruebas de umbral.
    #[cfg(test)]
    pub fn valid_for_days_for_test(days: u32) -> Result<Self, TlsError> {
        Self::made(days, Some(ChannelMark::Native))
    }

    fn made(validity_days: u32, mark: Option<ChannelMark>) -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, validity_days, mark).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// La marca de canal que lleva la CA local, o ninguna si es de antes de ella.
    pub fn mark(&self) -> Option<ChannelMark> {
        ChannelMark::of_certificate(&self.certificate.to_der().ok()?)
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

pub fn generate_key() -> Result<PKey<Private>, TlsError> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(not_generated)?;
    let key = EcKey::generate(&group).map_err(not_generated)?;
    PKey::from_ec_key(key).map_err(not_generated)
}

pub fn random_serial() -> Result<Asn1Integer, openssl::error::ErrorStack> {
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    serial.to_asn1_integer()
}

fn build_certificate(
    key: &PKey<Private>,
    validity_days: u32,
    mark: Option<ChannelMark>,
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
    if let Some(mark) = mark {
        builder.append_extension(channel_mark(mark)?)?;
    }
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

fn channel_mark(mark: ChannelMark) -> Result<X509Extension, openssl::error::ErrorStack> {
    const UTF8_STRING: u8 = 0x0c;

    let oid = Asn1Object::from_str(CHANNEL_MARK_OID)?;
    let contents = Asn1OctetString::new_from_bytes(&tagged(UTF8_STRING, mark.value().as_bytes()))?;
    X509Extension::new_from_der(&oid, false, &contents)
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
mod tests;
