use std::sync::Mutex;

use super::*;
use crate::identity::application::tests::a_certificate;
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

    fn sign(
        &self,
        _reference: &CertificateRef,
        pin: &str,
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
fn an_algorithm_the_token_does_not_offer_comes_back_as_a_situation() {
    let signer = RecordingSigner::default();
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    let refusal = signed_by_the_token(&signer, &certificate, "1234", "SHA512", b"uno")
        .expect_err("SHA512 no lo firma este token");

    assert_eq!(refusal.code, SafCode::CannotAccessKeystore);
    assert_eq!(refusal.situation, "unknown");
    assert!(refusal.detail.contains("SHA512"));
    assert!(crate::lock(&signer.signed).is_empty());
}

#[test]
fn the_algorithm_is_read_as_the_site_writes_it() {
    let signer = RecordingSigner::default();
    let certificate = a_certificate("FNMT-ACTIVO", b"der");

    for algorithm in [" SHA256 ", "sha256withrsa", "SHA256withRSA"] {
        signed_by_the_token(&signer, &certificate, "1234", algorithm, b"uno")
            .expect("el algoritmo de la sede se lee sin distinguir caja ni espacios");
    }
}

#[test]
fn a_token_that_cannot_sign_comes_back_with_its_code_and_its_situation() {
    struct AbsentToken;
    impl Signer for AbsentToken {
        fn secret_of(&self, _reference: &CertificateRef) -> Result<StoreSecret, TokenError> {
            Err(TokenError::new(Situation::TokenAbsent, "no hay token"))
        }

        fn sign(
            &self,
            _reference: &CertificateRef,
            _pin: &str,
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
