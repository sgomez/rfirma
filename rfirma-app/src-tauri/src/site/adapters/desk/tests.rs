use std::sync::Mutex;

use super::*;
use crate::identity::application::tests::a_certificate;
use crate::identity::domain::algorithm::{KeyKind, SignatureAlgorithm};
use crate::identity::domain::certificate::CertificateRef;

/// Un token que apunta con qué secreto y sobre qué bytes se le pidió cada firma.
#[derive(Default)]
struct RecordingSigner {
    signed: Mutex<Vec<(String, Vec<u8>)>>,
}

impl Signer for RecordingSigner {
    fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
        Ok(StoreSecret::TypedOnScreen {
            attempts_left: None,
        })
    }

    fn offers(
        &self,
        _reference: &CertificateRef,
        _algorithm: SignatureAlgorithm,
    ) -> Result<(), TokenError> {
        Ok(())
    }

    fn sign(
        &self,
        _reference: &CertificateRef,
        pin: &str,
        _algorithm: SignatureAlgorithm,
        data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        crate::lock(&self.signed).push((pin.to_owned(), data.to_vec()));
        Ok(b"PK1".to_vec())
    }
}

#[test]
fn one_secret_serves_every_signature_of_the_batch() {
    let signer = RecordingSigner::default();
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    let secret = secret_for_the_batch(&signer, &certificate).expect("el secreto deberia salir");
    for pre in [b"uno".as_slice(), b"dos".as_slice()] {
        signed_by_the_token(&signer, &certificate, "1234", "SHA256", pre).expect("firma");
    }

    assert!(matches!(secret, StoreSecret::TypedOnScreen { .. }));
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
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    let refusal = signed_by_the_token(&signer, &certificate, "1234", "RIPEMD160", b"uno")
        .expect_err("RIPEMD160 no lo compone rFirma");

    assert_eq!(refusal.code, SafCode::SignatureFailed);
    assert_eq!(refusal.situation, "mechanismNotOffered");
    assert!(refusal.detail.contains("RIPEMD160"));
    assert!(crate::lock(&signer.signed).is_empty());
}

#[test]
fn the_algorithm_is_read_as_the_site_writes_it() {
    let signer = RecordingSigner::default();
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    for algorithm in [
        " SHA256 ",
        "sha256withrsa",
        "SHA256withRSA",
        "SHA512withRSA",
    ] {
        signed_by_the_token(&signer, &certificate, "1234", algorithm, b"uno")
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
        assert_eq!(composed_for(asked, Some(KeyKind::Rsa)), rsa);
        assert_eq!(composed_for(asked, Some(KeyKind::Ec)), ec);
        assert_eq!(
            composed_for(asked, None),
            rsa,
            "sin clave legible se compone con RSA y el token dira que no"
        );
    }
}

#[test]
fn a_token_that_cannot_sign_comes_back_with_its_code_and_its_situation() {
    struct AbsentToken;
    impl Signer for AbsentToken {
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

        fn sign(
            &self,
            _reference: &CertificateRef,
            _pin: &str,
            _algorithm: SignatureAlgorithm,
            _data: &[u8],
        ) -> Result<Vec<u8>, TokenError> {
            Err(TokenError::new(Situation::TokenAbsent, "no hay token"))
        }
    }

    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    let refusal =
        secret_for_the_batch(&AbsentToken, &certificate).expect_err("sin token no hay secreto");

    assert_eq!(refusal.code, SafCode::CannotFindKeystore);
    assert_eq!(refusal.situation, "tokenAbsent");
}

#[test]
fn the_suffix_declared_by_the_site_is_ignored_and_the_certificate_key_class_rules() {
    let asked = AskedAlgorithm::named("SHA256withRSA").expect("es SHA256");
    assert_eq!(
        composed_for(asked, Some(KeyKind::Ec)),
        SignatureAlgorithm::Sha256Ecdsa
    );

    let asked_ecdsa = AskedAlgorithm::named("SHA384withECDSA").expect("es SHA384");
    assert_eq!(
        composed_for(asked_ecdsa, Some(KeyKind::Rsa)),
        SignatureAlgorithm::Sha384Rsa
    );

    let asked_hyphen = AskedAlgorithm::named("SHA-512withRSA").expect("es SHA512");
    assert_eq!(
        composed_for(asked_hyphen, Some(KeyKind::Ec)),
        SignatureAlgorithm::Sha512Ecdsa
    );
}
