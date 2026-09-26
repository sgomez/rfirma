use std::sync::Mutex;

use super::*;
use crate::identity::application::tests::{a_certificate, a_usable_certificate};
use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::protected_secret::ProtectedSecret;

/// Un token que apunta con qué secreto y sobre qué bytes se le pidió cada firma.
#[derive(Default)]
struct RecordingSigner {
    signed: Mutex<Vec<(String, Vec<u8>)>>,
}

impl Signer for RecordingSigner {
    fn accepts_the_secret(
        &self,
        _reference: &crate::identity::domain::certificate::CertificateRef,
        _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
    ) -> Result<(), crate::identity::domain::error::TokenError> {
        Ok(())
    }

    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::TypedOnScreen)
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Ok(())
    }

    fn sign_with_secret(
        &self,
        _reference: &CertificateRef,
        secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        _algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        let pin = secret.as_str().expect("PIN de prueba en UTF-8").to_owned();
        crate::lock(&self.signed).push((pin, data.to_vec()));
        Ok(b"PK1".to_vec())
    }
}

#[test]
fn one_secret_serves_every_signature_of_the_batch() {
    let signer = RecordingSigner::default();
    let certificate = a_usable_certificate("FNMT-ACTIVO");

    let secret =
        secret_for_the_remote_batch(&signer, &certificate).expect("el secreto deberia salir");
    let pin = ProtectedSecret::from_str("1234");
    for pre in [b"uno".as_slice(), b"dos".as_slice()] {
        signed_for_the_remote_batch(&signer, &certificate, &pin, "SHA256", pre).expect("firma");
    }

    assert!(matches!(secret, StoreSecret::TypedOnScreen));
    assert_eq!(
        *crate::lock(&signer.signed),
        vec![
            ("1234".to_owned(), b"uno".to_vec()),
            ("1234".to_owned(), b"dos".to_vec()),
        ]
    );
}

#[test]
fn an_algorithm_rfirma_does_not_compose_comes_back_with_the_code_of_the_original() {
    let signer = RecordingSigner::default();
    let certificate = a_usable_certificate("FNMT-ACTIVO");

    let refusal = signed_for_the_remote_batch(
        &signer,
        &certificate,
        &ProtectedSecret::from_str("1234"),
        "RIPEMD160",
        b"uno",
    )
    .expect_err("RIPEMD160 no lo compone rFirma");

    assert_eq!(refusal.code, SafCode::SignatureFailed);
    assert_eq!(refusal.situation, "mechanismNotOffered");
    assert!(refusal.detail.contains("RIPEMD160"));
    assert!(crate::lock(&signer.signed).is_empty());
}

#[test]
fn the_algorithm_is_read_as_the_site_writes_it() {
    let signer = RecordingSigner::default();
    let certificate = a_usable_certificate("FNMT-ACTIVO");

    let pin = ProtectedSecret::from_str("1234");
    for algorithm in [
        " SHA256 ",
        "sha256withrsa",
        "SHA256withRSA",
        "SHA512withRSA",
    ] {
        signed_for_the_remote_batch(&signer, &certificate, &pin, algorithm, b"uno")
            .expect("el algoritmo de la sede se lee sin distinguir caja ni espacios");
    }
}

#[test]
fn the_digest_the_site_asks_for_is_composed_with_the_key_of_the_certificate() {
    for (asked, rsa, ec) in [
        (
            AskedAlgorithm::Sha256,
            SignatureAlgorithm::Sha256Rsa,
            SignatureAlgorithm::Sha256Ecdsa,
        ),
        (
            AskedAlgorithm::Sha384,
            SignatureAlgorithm::Sha384Rsa,
            SignatureAlgorithm::Sha384Ecdsa,
        ),
        (
            AskedAlgorithm::Sha512,
            SignatureAlgorithm::Sha512Rsa,
            SignatureAlgorithm::Sha512Ecdsa,
        ),
    ] {
        assert_eq!(composed_for(asked, Some(KeyKind::Rsa)), Ok(rsa));
        assert_eq!(composed_for(asked, Some(KeyKind::Ec)), Ok(ec));
    }
}

#[test]
fn a_key_neither_rsa_nor_ec_is_refused_as_an_incompatible_key_type() {
    let refusal = composed_for(AskedAlgorithm::Sha256, None)
        .map_err(refusal_of_token)
        .expect_err("una clave desconocida no se compone con RSA");

    assert_eq!(refusal.code, SafCode::IncompatibleKeyType);
    assert_eq!(refusal.situation, "keyNotRsa");
}

#[test]
fn a_token_that_cannot_sign_comes_back_with_its_code_and_its_situation() {
    struct AbsentToken;
    impl Signer for AbsentToken {
        fn accepts_the_secret(
            &self,
            _reference: &crate::identity::domain::certificate::CertificateRef,
            _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
        ) -> Result<(), crate::identity::domain::error::TokenError> {
            Ok(())
        }

        fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
            Err(TokenError::new(Situation::TokenAbsent, "no hay token"))
        }

        fn offers(
            &self,
            _reference: &CertificateRef,
            _algorithm: SignatureAlgorithm,
        ) -> Result<(), TokenError> {
            Err(TokenError::new(Situation::TokenAbsent, "no hay token"))
        }

        fn sign_with_secret(
            &self,
            _reference: &CertificateRef,
            _secret: &crate::identity::domain::protected_secret::ProtectedSecret,
            _algorithm: SignatureAlgorithm,
            _data: &[u8],
        ) -> Result<Vec<u8>, TokenError> {
            Err(TokenError::new(Situation::TokenAbsent, "no hay token"))
        }
    }

    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    let refusal = secret_for_the_remote_batch(&AbsentToken, &certificate)
        .expect_err("sin token no hay secreto");

    assert_eq!(refusal.code, SafCode::CannotAccessKeystore);
    assert_eq!(refusal.situation, "tokenAbsent");
}

#[test]
fn the_suffix_declared_by_the_site_is_ignored_and_the_certificate_key_class_rules() {
    let asked = AskedAlgorithm::named("SHA256withRSA").expect("es SHA256");
    assert_eq!(
        composed_for(asked, Some(KeyKind::Ec)),
        Ok(SignatureAlgorithm::Sha256Ecdsa)
    );

    let asked_ecdsa = AskedAlgorithm::named("SHA384withECDSA").expect("es SHA384");
    assert_eq!(
        composed_for(asked_ecdsa, Some(KeyKind::Rsa)),
        Ok(SignatureAlgorithm::Sha384Rsa)
    );

    let asked_hyphen = AskedAlgorithm::named("SHA-512withRSA").expect("es SHA512");
    assert_eq!(
        composed_for(asked_hyphen, Some(KeyKind::Ec)),
        Ok(SignatureAlgorithm::Sha512Ecdsa)
    );
}
