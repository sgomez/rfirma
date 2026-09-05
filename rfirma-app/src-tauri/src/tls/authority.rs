//! La **CA local**: la pieza que se conserva entre arranques (ID-220, ID-221,
//! ADR-0005).
//!
//! No identifica a nadie ni firma documentos. Su único trabajo es firmar el
//! **certificado del servidor local**, y por eso lleva encima todo lo que la
//! hace inofensiva si un día queda abandonada en el `$HOME` de alguien:
//!
//! - `basicConstraints` crítica, `CA:TRUE` con `pathlen:0`, así que no puede
//!   emitir otra CA por debajo;
//! - `keyUsage` crítica reducida a `keyCertSign` y `cRLSign`: no sirve para
//!   TLS ni para firmar nada que no sea un certificado;
//! - `nameConstraints` crítica, que la acota a `localhost`, `127.0.0.1` y
//!   `::1` —el #310 midió que los tres motores la imponen de verdad, incluida
//!   la restricción sobre `iPAddress`, y con la violación visible y no
//!   salteable—;
//! - y una caducidad de [`VALIDITY_DAYS`], dentro de la banda de 2–3 años del
//!   ID-221. **El acotamiento por forma no sustituye a la caducidad**: la
//!   caducidad es lo único que hace inerte un residuo que nadie borra, porque
//!   un `apt remove` no toca el `$HOME`.
//!
//! **La clave es de curva elíptica P-256**, no RSA: la firma la hace OpenSSL y
//! no Rust —el ID-225 descarta `x509-cert` justo por eso—, y generar una clave
//! P-256 es instantáneo, cosa que importa porque el certificado del servidor
//! local se genera **en cada arranque** (ID-222) y el arranque lo dispara la
//! sede. P-256 con ECDSA-SHA256 es el mínimo común de cualquier TLS 1.2 o 1.3.

use openssl::asn1::{Asn1Integer, Asn1Object, Asn1OctetString, Asn1Time};
use openssl::bn::{BigNum, MsbOption};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::extension::{BasicConstraints, KeyUsage, SubjectKeyIdentifier};
use openssl::x509::{X509Extension, X509Name, X509};

use super::error::{Situation, TlsError};

/// Cuánto vive la CA local, dentro de la banda de 2–3 años del ID-221.
///
/// El número exacto es un juicio y el ADR-0005 lo dice: los dos extremos valen
/// igual. 900 días son dos años y medio largos, que deja sitio de sobra para
/// que el solape instale la siguiente meses antes de que esta caduque.
pub const VALIDITY_DAYS: u32 = 900;

/// El `CN` de la CA local. **No es un identificador**: la retirada borra por
/// huella del certificado y nunca por *nickname* (ADR-0005, punto 6).
pub const COMMON_NAME: &str = "rFirma CA local";

/// El nombre DNS al que queda acotada la CA local.
pub const PERMITTED_DNS_NAME: &str = "localhost";
/// La dirección IPv4 de *loopback*, la que `autoscript.js` trae cableada.
pub const PERMITTED_IPV4: [u8; 4] = [127, 0, 0, 1];
/// La dirección IPv6 de *loopback*.
pub const PERMITTED_IPV6: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];

/// La CA local: su certificado y su clave privada, que **sí** se conserva.
#[derive(Clone)]
pub struct LocalCa {
    certificate: X509,
    key: PKey<Private>,
}

impl LocalCa {
    /// Fabrica una CA local nueva, válida desde ahora mismo.
    pub fn generate() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, VALIDITY_DAYS).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// Una CA local a la que le quedan **dos días**, para poder ejercitar el
    /// solape sin esperar dos años y medio (ID-224).
    ///
    /// Dos y no uno **a propósito**: [`LocalCa::days_left`] redondea hacia
    /// abajo, así que con un día bastaría **un segundo** entre fabricarla y
    /// preguntarle —guardar dos ficheros, registrar en dos perfiles— para que
    /// devolviera `0` y la etapa saliera caducada en vez de en solape. Con dos
    /// días la cuenta da 1 o 2, y las dos están dentro del solape, que llega
    /// hasta 119.
    #[cfg(test)]
    pub fn almost_expired_for_test() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, 2).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// Una CA local que **ya no vale**: caduca hoy mismo, así que
    /// [`LocalCa::days_left`] devuelve `0` y la etapa es
    /// [`crate::trust::Stage::Expired`]. No hay carrera posible: el número solo
    /// puede bajar.
    #[cfg(test)]
    pub fn expired_for_test() -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = build_certificate(&key, 0).map_err(not_generated)?;
        Ok(Self { certificate, key })
    }

    /// La CA local que había guardada, a partir de los dos PEM.
    ///
    /// Un par que no se corresponde es material dañado y no medio bueno: se
    /// comprueba aquí y no cuando el saludo TLS falle sin explicación.
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

    /// El certificado, que es lo que se registra en los almacenes NSS.
    pub fn certificate(&self) -> &X509 {
        &self.certificate
    }

    /// La clave privada, que **solo** sirve para firmar el certificado del
    /// servidor local.
    pub fn key(&self) -> &PKey<Private> {
        &self.key
    }

    /// El certificado en PEM, tal y como se guarda en disco.
    pub fn certificate_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.certificate.to_pem().map_err(not_generated)
    }

    /// La clave privada en PEM PKCS#8 y **sin cifrar**, tal y como se guarda en
    /// el fichero `0600` (ID-223).
    ///
    /// Sin cifrar a propósito: la contraseña tendría que vivir en el código, y
    /// el ADR-0005 recuerda que `RestoreConfigLinux.java` de AutoFirma lleva
    /// `KS_PASSWORD = "654321"` en su fuente público.
    pub fn private_key_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.key.private_key_to_pem_pkcs8().map_err(not_generated)
    }

    /// Cuántos días le quedan de vida, **negativo si ya caducó**.
    ///
    /// Es lo único que hace falta para decidir el solape (ID-221, ID-224), y
    /// por eso sale como número y no como fecha: quien decide es
    /// [`crate::trust::Stage`], que es puro y no sabe qué hora es.
    ///
    /// **Redondea hacia abajo**: se queda con los días enteros de la diferencia
    /// y tira los segundos, así que a una CA a la que le quedan veintitrés
    /// horas le devuelve `0` y [`crate::trust::Stage`] la da por caducada. El
    /// error es siempre conservador —se releva hasta un día antes de tiempo,
    /// nunca después—, pero conviene saberlo al leer
    /// [`crate::trust::Stage::Expired`]: el último día cuenta como caducado.
    pub fn days_left(&self) -> Result<i64, TlsError> {
        let now = Asn1Time::days_from_now(0).map_err(damaged)?;
        let difference = now.diff(self.certificate.not_after()).map_err(damaged)?;
        Ok(i64::from(difference.days))
    }
}

impl std::fmt::Debug for LocalCa {
    /// A mano, y sin la clave: un `derive` la metería en cualquier registro que
    /// imprima la estructura.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCa")
            .field("not_after", &self.certificate.not_after().to_string())
            .finish_non_exhaustive()
    }
}

/// Una clave P-256 nueva. La comparten la CA local y el certificado del
/// servidor local, que es la única cosa que tienen en común.
pub(super) fn generate_key() -> Result<PKey<Private>, TlsError> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).map_err(not_generated)?;
    let key = EcKey::generate(&group).map_err(not_generated)?;
    PKey::from_ec_key(key).map_err(not_generated)
}

/// Un número de serie de 20 bytes sacado al azar, como pide el CA/Browser
/// Forum y como espera cualquier verificador moderno.
pub(super) fn random_serial() -> Result<Asn1Integer, openssl::error::ErrorStack> {
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    serial.to_asn1_integer()
}

/// El certificado autofirmado de la CA local.
fn build_certificate(
    key: &PKey<Private>,
    validity_days: u32,
) -> Result<X509, openssl::error::ErrorStack> {
    let mut name = X509Name::builder()?;
    name.append_entry_by_nid(Nid::COMMONNAME, COMMON_NAME)?;
    let name = name.build();

    let mut builder = X509::builder()?;
    // `2` es la v3: la numeración del campo empieza en cero. Sin v3 no hay
    // extensiones, y esta CA es sus extensiones.
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

/// La extensión `nameConstraints`, **crítica**, armada byte a byte.
///
/// El crate `openssl` no tiene constructor para esta extensión, y el que sí
/// valdría —`X509Extension::new_nid`, que acepta la sintaxis de `openssl.cnf`—
/// está obsoleto y el `clippy` del proyecto va con `-D warnings`. Así que se
/// escribe el DER, que además es más honesto: lo que acaba dentro del
/// certificado se lee aquí y no en una cadena de configuración.
///
/// ```text
/// NameConstraints ::= SEQUENCE { permittedSubtrees [0] GeneralSubtrees }
/// GeneralSubtrees ::= SEQUENCE OF GeneralSubtree
/// GeneralSubtree  ::= SEQUENCE { base GeneralName }
/// GeneralName     ::= dNSName [2] IA5String | iPAddress [7] OCTET STRING
/// ```
///
/// La dirección va **con su máscara detrás** —ocho bytes en IPv4, treinta y dos
/// en IPv6—, que es lo que distingue una restricción de un `subjectAltName` y
/// lo que el #310 midió que los tres motores aplican de verdad.
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

    // 2.5.29.30 es `id-ce-nameConstraints` (RFC 5280, § 4.2.1.10).
    let oid = Asn1Object::from_str("2.5.29.30")?;
    let contents = Asn1OctetString::new_from_bytes(&der)?;
    X509Extension::new_from_der(&oid, true, &contents)
}

/// Un valor DER con su etiqueta y su longitud en forma corta.
///
/// La forma corta llega hasta 127 bytes y aquí lo más grande son los sesenta y
/// pico de `nameConstraints`, que además son constantes: no hay ninguna entrada
/// de fuera que pueda hacer crecer esto. El `assert!` está para que, si alguien
/// añade un nombre permitido y se pasa, falle aquí y no emita un certificado
/// silenciosamente mal formado.
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
            "una caducidad mas corta que el hueco entre dos usos garantiza el camino malo (ID-222)"
        );
        assert!(
            ca.certificate().not_after() < three_years.as_ref(),
            "la caducidad es la unica red contra un residuo abandonado (ID-221)"
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
        assert!(
            !key.contains("ENCRYPTED"),
            "va sin cifrar a proposito: la contrasena tendria que vivir en el codigo (ID-223)"
        );
    }

    #[test]
    fn the_debug_output_never_carries_the_private_key() {
        let ca = LocalCa::generate().expect("deberia generarse");

        let printed = format!("{ca:?}");

        assert!(printed.contains("LocalCa"), "{printed}");
        assert!(!printed.contains("PRIVATE"), "{printed}");
    }
}
