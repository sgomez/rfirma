//! Fixtures del token de pruebas y ayudantes de firma/verificación para `pkcs11_token.rs`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use openssl::hash::MessageDigest;
use openssl::rsa::Padding;
use openssl::sign::{RsaPssSaltlen, Verifier as OpensslVerifier};
use openssl::x509::X509;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::domain::algorithm::SignatureAlgorithm;
use rfirma_lib::identity::domain::certificate::{CertificateRef, TokenCertificate};
use rfirma_lib::identity::domain::error::TokenError;
use rsa::pkcs1v15::VerifyingKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use sha2::Sha256;
use x509_cert::der::{Decode, Encode};

pub(crate) const TOKEN: &str = "rfirma-test";
pub(crate) const PIN: &str = "1234";
pub(crate) const ACTIVE: &str = "FNMT-ACTIVO-99999999R";
/// El único certificado del token de curva elíptica.
pub(crate) const ACTIVE_EC: &str = "FNMT-ACTIVO-ECC-99949991H";
pub(crate) const EXPIRED: &str = "FNMT-CADUCADO-99999999R";
pub(crate) const REVOKED: &str = "FNMT-REVOCADO-99999999R";
/// Dos certificados que comparten etiqueta y no comparten clave.
pub(crate) const TWIN: &str = "FNMT-GEMELO-99999999R";
pub(crate) const TWIN_OF_THE_ACTIVE_KEY: u8 = 0x04;
pub(crate) const TWIN_OF_THE_EXPIRED_KEY: u8 = 0x05;

/// Bloque DER de SignedAttributes para firmar.
pub(crate) const PRESIGN: &[u8] =
    b"31 5f 30 18 06 09 2a 86 SignedAttributes de mentira, sin hashear";

pub(crate) fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just certs install",
        module.display()
    );
    module
}

pub(crate) fn certificates() -> Vec<TokenCertificate> {
    let found = pkcs11::list_certificates(module()).expect("no se ha podido listar el token");
    assert!(
        !found.is_empty(),
        "el token {TOKEN} esta vacio o no existe. Montalo con:\n  just certs install"
    );
    found
}

pub(crate) fn certificate_labelled(label: &str) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().label() == label)
        .unwrap_or_else(|| {
            panic!("el token {TOKEN} no tiene ningun certificado {label}. Montalo con: just certs install")
        })
}

/// Referencia tal y como sale del token con su CKA_ID.
pub(crate) fn reference(label: &str) -> CertificateRef {
    certificate_labelled(label).reference().clone()
}

pub(crate) fn certificate_with_cka_id(cka_id: u8) -> TokenCertificate {
    certificates()
        .into_iter()
        .find(|certificate| certificate.reference().cka_id() == Some([cka_id].as_slice()))
        .unwrap_or_else(|| {
            panic!(
                "el token {TOKEN} no tiene ningun certificado con CKA_ID {cka_id:02x}. \
                 Montalo con: just certs install"
            )
        })
}

pub(crate) fn epoch(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

pub(crate) fn verifying_key(certificate: &TokenCertificate) -> VerifyingKey<Sha256> {
    let parsed =
        x509_cert::Certificate::from_der(certificate.der()).expect("el DER deberia parsearse");
    let spki = parsed
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .expect("el SPKI deberia serializarse");
    let public_key = RsaPublicKey::from_public_key_der(&spki).expect("clave publica RSA");
    VerifyingKey::<Sha256>::new(public_key)
}

/// Mecanismo CKM_RSA_PKCS invocado a mano como contraejemplo.
pub(crate) fn sign_with_bare_rsa_pkcs(data: &[u8]) -> Vec<u8> {
    pkcs11::with_token_turn(|| sign_with_bare_rsa_pkcs_holding_the_turn(data))
}

fn sign_with_bare_rsa_pkcs_holding_the_turn(data: &[u8]) -> Vec<u8> {
    use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
    use cryptoki::mechanism::Mechanism;
    use cryptoki::object::{Attribute, ObjectClass};
    use cryptoki::session::UserType;
    use cryptoki::types::AuthPin;

    let context = Pkcs11::new(module()).expect("modulo");
    let _ = context.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK));

    let slot = context
        .get_slots_with_token()
        .expect("ranuras")
        .into_iter()
        .find(|slot| {
            context
                .get_token_info(*slot)
                .map(|info| info.label().trim() == TOKEN)
                .unwrap_or(false)
        })
        .expect("el token rfirma-test");

    let session = context.open_ro_session(slot).expect("sesion");
    session
        .login(UserType::User, Some(&AuthPin::new(PIN.into())))
        .expect("el login del contraejemplo, con el turno del token cogido");
    let key = session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Label(ACTIVE.as_bytes().to_vec()),
        ])
        .expect("busqueda")
        .into_iter()
        .next()
        .expect("la clave del camino feliz");

    let signature = session
        .sign(&Mechanism::RsaPkcs, key, data)
        .expect("CKM_RSA_PKCS deberia firmar cualquier bloque que le quepa");

    let _ = session.logout();

    signature
}

/// Verifica con OpenSSL, el mismo contraste que hara despues un validador CAdES.
pub(crate) fn openssl_verifies(digest: MessageDigest, padding: Padding, signature: &[u8]) -> bool {
    openssl_verifies_for(ACTIVE, digest, Some(padding), signature)
}

/// Verifica con OpenSSL contra la clave pública del certificado que se le diga.
pub(crate) fn openssl_verifies_for(
    label: &str,
    digest: MessageDigest,
    padding: Option<Padding>,
    signature: &[u8],
) -> bool {
    let certificate = certificate_labelled(label);
    let parsed = X509::from_der(certificate.der()).expect("el DER deberia parsearse");
    let public_key = parsed.public_key().expect("clave publica del certificado");

    let mut verifier = OpensslVerifier::new(digest, &public_key).expect("verificador de OpenSSL");
    if padding == Some(Padding::PKCS1_PSS) {
        verifier
            .set_rsa_padding(Padding::PKCS1_PSS)
            .expect("relleno PSS");
        verifier
            .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
            .expect("sal del tamano del resumen");
        verifier
            .set_rsa_mgf1_md(digest)
            .expect("MGF1 con el resumen");
    }
    verifier.update(PRESIGN).expect("los bytes sin hashear");

    verifier.verify(signature).expect("la verificacion corre")
}

pub(crate) fn signing_error(reference: &CertificateRef, pin: &str) -> TokenError {
    pkcs11::sign(reference, pin, SignatureAlgorithm::Sha256Rsa, PRESIGN)
        .expect_err("esto tenia que fallar")
}
