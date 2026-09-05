//! El **certificado del servidor local**: la pieza que no se guarda (ID-220,
//! ID-222, ADR-0005).
//!
//! Se genera en memoria en cada arranque y vive lo que vive el proceso. No hay
//! ruta para él en [`crate::paths`] y no la habrá: reemitirlo no toca el
//! `nssdb` —el punto de confianza es la CA local, y este viaja en el saludo
//! TLS—, así que rehacerlo es gratis y no interrumpe a nadie.
//!
//! Su forma es **exacta y no es una preferencia**: `CN=localhost` y las **dos**
//! entradas en la SAN, `DNS:localhost` e `IP:127.0.0.1`. Es la única que pasa
//! en los cuatro verificadores medidos en el #310 y el #326, y `autoscript.js`
//! —que sirve la sede, no nosotros— trae cableado
//! `SERVER_HOST = "127.0.0.1"`, así que la entrada de IP no es un extra: es por
//! donde llega la conexión.

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

/// El `CN` del certificado del servidor local.
pub const COMMON_NAME: &str = PERMITTED_DNS_NAME;

/// Cuánto vive el certificado del servidor local.
///
/// Es un número sin consecuencias —se rehace en cada arranque, así que nadie lo
/// ve caducar— y por eso es corto: si alguna vez sobrevive a un proceso, que no
/// sobreviva mucho. Los 90 días que el ADR-0005 llegó a escribir eran de la
/// época en la que esta pieza **sí** se guardaba, y eran el agujero.
pub const VALIDITY_DAYS: u32 = 30;

/// El certificado que rfirma presenta en cada saludo TLS, con su clave.
///
/// Ninguno de los dos toca el disco.
#[derive(Clone)]
pub struct LocalServerCertificate {
    certificate: X509,
    key: PKey<Private>,
}

impl LocalServerCertificate {
    /// Fabrica el certificado del servidor local y lo firma con la CA local.
    pub fn issued_by(ca: &LocalCa) -> Result<Self, TlsError> {
        let key = generate_key()?;
        let certificate = issue(ca, &key, COMMON_NAME, |names| {
            names.dns(PERMITTED_DNS_NAME).ip("127.0.0.1");
        })
        .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))?;
        Ok(Self { certificate, key })
    }

    /// El certificado, que es lo que va en el saludo TLS.
    pub fn certificate(&self) -> &X509 {
        &self.certificate
    }

    /// La clave privada, que solo existe en memoria.
    pub fn key(&self) -> &PKey<Private> {
        &self.key
    }

    /// El certificado en PEM, para dárselo al servidor.
    pub fn certificate_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.certificate
            .to_pem()
            .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))
    }

    /// La clave privada en PEM PKCS#8, para dársela al servidor. **No se
    /// escribe en ningún fichero** (ID-222).
    pub fn private_key_pem(&self) -> Result<Vec<u8>, TlsError> {
        self.key
            .private_key_to_pem_pkcs8()
            .map_err(|error| TlsError::new(Situation::MaterialNotGenerated, error.to_string()))
    }
}

impl std::fmt::Debug for LocalServerCertificate {
    /// A mano y sin la clave, por lo mismo que en [`LocalCa`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalServerCertificate")
            .field("not_after", &self.certificate.not_after().to_string())
            .finish_non_exhaustive()
    }
}

/// Emite un certificado de servidor firmado por la CA local.
///
/// Toma el nombre y las entradas de la SAN por parámetro **solo** para que las
/// pruebas puedan pedirle a la CA local algo que no debe conceder: en
/// producción hay una sola llamada y trae las constantes de este módulo.
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
mod tests {
    use openssl::stack::Stack;
    use openssl::x509::store::X509StoreBuilder;
    use openssl::x509::{X509StoreContext, X509VerifyResult};

    use super::*;

    /// Verifica la cadena como lo haría el navegador: el certificado del
    /// servidor local contra un almacén que solo contiene la CA local.
    fn verdict(ca: &LocalCa, certificate: &X509) -> X509VerifyResult {
        let mut store = X509StoreBuilder::new().expect("deberia haber almacen");
        store
            .add_cert(ca.certificate().clone())
            .expect("deberia entrar la CA local");
        let store = store.build();

        let chain = Stack::new().expect("deberia haber pila");
        let mut context = X509StoreContext::new().expect("deberia haber contexto");
        context
            .init(&store, certificate, &chain, |context| {
                let _ = context.verify_cert();
                Ok(context.error())
            })
            .expect("deberia verificarse")
    }

    #[test]
    fn the_sede_reaches_the_local_server_by_name_and_by_address() {
        let ca = LocalCa::generate().expect("deberia generarse");
        let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

        let text = String::from_utf8(server.certificate().to_text().expect("deberia imprimirse"))
            .expect("deberia ser UTF-8");

        let common_name = server
            .certificate()
            .subject_name()
            .entries_by_nid(Nid::COMMONNAME)
            .next()
            .expect("deberia haber CN")
            .data()
            .to_string()
            .expect("deberia ser UTF-8");
        assert_eq!(common_name, "localhost");
        assert!(
            text.contains("DNS:localhost, IP Address:127.0.0.1"),
            "hacen falta las DOS entradas en la SAN (ID-222):\n{text}"
        );
    }

    #[test]
    fn a_browser_that_trusts_the_local_ca_accepts_the_local_server_certificate() {
        let ca = LocalCa::generate().expect("deberia generarse");
        let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

        assert_eq!(verdict(&ca, server.certificate()), X509VerifyResult::OK);
    }

    #[test]
    fn the_local_ca_cannot_vouch_for_a_site_outside_the_loopback() {
        let ca = LocalCa::generate().expect("deberia generarse");
        let key = generate_key().expect("deberia generarse");

        let impostor = issue(&ca, &key, "sede.example", |names| {
            names.dns("sede.example");
        })
        .expect("emitirlo se puede: quien lo rechaza es el verificador");

        let verdict = verdict(&ca, &impostor);
        assert_ne!(
            verdict,
            X509VerifyResult::OK,
            "la restriccion de nombres es lo que hace inofensiva una CA local abandonada"
        );
        assert!(
            verdict.error_string().contains("permitted subtree"),
            "lo rechaza `nameConstraints` y no otra cosa: {}",
            verdict.error_string()
        );
    }

    #[test]
    fn the_local_server_certificate_is_not_an_authority() {
        let ca = LocalCa::generate().expect("deberia generarse");
        let server = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

        let text = String::from_utf8(server.certificate().to_text().expect("deberia imprimirse"))
            .expect("deberia ser UTF-8");

        assert!(text.contains("CA:FALSE"), "{text}");
        assert!(text.contains("TLS Web Server Authentication"), "{text}");
    }

    #[test]
    fn every_boot_gets_a_brand_new_local_server_certificate() {
        let ca = LocalCa::generate().expect("deberia generarse");

        let one = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");
        let another = LocalServerCertificate::issued_by(&ca).expect("deberia emitirse");

        assert_ne!(
            one.private_key_pem().expect("deberia salir en PEM"),
            another.private_key_pem().expect("deberia salir en PEM"),
            "no se guarda, asi que tampoco se reutiliza (ID-222)"
        );
    }
}
