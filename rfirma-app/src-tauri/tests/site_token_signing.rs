//! Pruebas de integración del puerto de firma del lote contra el módulo PKCS#11 SoftHSM (ADR-0014).

use std::path::PathBuf;

use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Verifier;
use rfirma_lib::identity::adapters::pkcs11::{self, RealToken};
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::secret::StoreSecret;
use rfirma_lib::site::adapters::desk::{secret_for_the_batch, signed_by_the_token};
use rfirma_lib::site::domain::protocol::SafCode;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier as _;
use rsa::RsaPublicKey;
use sha2::{Sha256, Sha512};
use x509_cert::der::{Decode, Encode};

const TOKEN: &str = "rfirma-test";
const PIN: &str = "1234";
const ACTIVE: &str = "FNMT-ACTIVO-99999999R";

/// Dos prefirmas distintas del mismo lote.
const FIRST: &[u8] = b"31 5f 30 18 SignedAttributes del primer documento del lote";
const SECOND: &[u8] = b"31 5f 30 18 SignedAttributes del segundo documento del lote";

fn module() -> PathBuf {
    let module = PathBuf::from(
        std::env::var("RFIRMA_PKCS11_MODULE")
            .unwrap_or_else(|_| "/usr/lib/softhsm/libsofthsm2.so".to_owned()),
    );
    assert!(
        module.is_file(),
        "falta el modulo PKCS#11 en {}. Las pruebas de grada B necesitan SoftHSM:\n  \
         sudo apt install -y softhsm2 opensc\n  just token",
        module.display()
    );
    module
}

fn certificate() -> TokenCertificate {
    let found = pkcs11::list_certificates(module()).expect("no se ha podido listar el token");
    found
        .into_iter()
        .find(|certificate| certificate.reference().label() == ACTIVE)
        .unwrap_or_else(|| panic!("falta {ACTIVE} en el token {TOKEN}. Montalo con:\n  just token"))
}

fn public_key(certificate: &TokenCertificate) -> RsaPublicKey {
    let parsed =
        x509_cert::Certificate::from_der(certificate.der()).expect("el DER deberia parsearse");
    let spki = parsed
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .expect("el SPKI deberia serializarse");
    RsaPublicKey::from_public_key_der(&spki).expect("clave publica RSA")
}

fn verifies_with_sha1(certificate: &TokenCertificate, data: &[u8], signature: &[u8]) -> bool {
    let parsed =
        x509_cert::Certificate::from_der(certificate.der()).expect("el DER deberia parsearse");
    let spki = parsed
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .expect("el SPKI deberia serializarse");
    let key = PKey::public_key_from_der(&spki).expect("clave publica del certificado");

    let mut verifier =
        Verifier::new(MessageDigest::sha1(), &key).expect("openssl deberia ofrecer SHA1 con RSA");
    verifier.update(data).expect("los datos deberian entrar");
    verifier.verify(signature).expect("la firma deberia leerse")
}

#[test]
fn one_secret_signs_the_whole_batch_and_every_signature_verifies() {
    let certificate = certificate();
    let signer = RealToken;

    let secret = secret_for_the_batch(&signer, &certificate).expect("el secreto deberia salir");
    assert!(matches!(secret, StoreSecret::TypedOnScreen { .. }));

    let key = VerifyingKey::<Sha256>::new(public_key(&certificate));
    for pre in [FIRST, SECOND] {
        let raw = signed_by_the_token(&signer, &certificate, PIN, "SHA256", pre)
            .expect("la firma deberia salir");
        let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
        key.verify(pre, &signature)
            .expect("la firma no verifica contra la clave publica del certificado");
    }
}

#[test]
fn the_sha512_the_site_asks_for_is_signed_by_the_token_and_verifies() {
    let certificate = certificate();

    let raw = signed_by_the_token(&RealToken, &certificate, PIN, "SHA512withRSA", FIRST)
        .expect("el token ofrece CKM_SHA512_RSA_PKCS");

    let signature = Signature::try_from(raw.as_slice()).expect("firma RSA");
    VerifyingKey::<Sha512>::new(public_key(&certificate))
        .verify(FIRST, &signature)
        .expect("la firma SHA512 no verifica contra la clave publica del certificado");
}

#[test]
fn the_sha1_a_site_still_asks_for_is_signed_by_the_token_and_verifies() {
    let certificate = certificate();

    let raw = signed_by_the_token(&RealToken, &certificate, PIN, "SHA1withRSA", FIRST)
        .expect("el token ofrece CKM_SHA1_RSA_PKCS");

    assert!(
        verifies_with_sha1(&certificate, FIRST, &raw),
        "la firma SHA1 no verifica contra la clave publica del certificado"
    );
}

#[test]
fn an_algorithm_rfirma_does_not_compose_is_a_situation_and_not_a_panic() {
    let certificate = certificate();

    let refusal = signed_by_the_token(&RealToken, &certificate, PIN, "RIPEMD160withRSA", FIRST)
        .expect_err("rFirma no compone RIPEMD160");

    assert_eq!(refusal.code, SafCode::SignatureFailed);
    assert_eq!(refusal.situation, "mechanismNotOffered");
    assert!(refusal.detail.contains("RIPEMD160withRSA"));
}

#[test]
fn a_wrong_secret_does_not_sign_the_rest_of_the_batch() {
    let certificate = certificate();

    let refusal = signed_by_the_token(&RealToken, &certificate, "9999", "SHA256", FIRST)
        .expect_err("un PIN que no es el del token no firma");

    assert_eq!(refusal.situation, "incorrectPin");
    assert_eq!(refusal.code, SafCode::CannotAccessKeystore);
}
