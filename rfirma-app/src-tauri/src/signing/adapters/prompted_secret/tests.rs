use super::{secret_for_the_batch, sign_on_token_with_prompter, signed_on_the_token};
use crate::identity::application::tests::a_certificate;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::signing::adapters::gtk_prompter::{MockSecretPrompter, PreconfiguredSecretPrompter};
use crate::signing::application::session::SigningSession;
use crate::signing::domain::Language;
use crate::signing::ports::Signer;

/// Un token que pide el PIN en pantalla y solo acepta `1234`.
struct ATokenThatAcceptsOnly1234;

impl Signer for ATokenThatAcceptsOnly1234 {
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
        _secret: &ProtectedSecret,
        _algorithm: SignatureAlgorithm,
        _data: &[u8],
    ) -> Result<Vec<u8>, TokenError> {
        unreachable!("pedir el secreto del lote no firma nada")
    }

    fn accepts_the_secret(
        &self,
        _reference: &CertificateRef,
        secret: &ProtectedSecret,
    ) -> Result<(), TokenError> {
        if secret.as_bytes() == b"1234" {
            return Ok(());
        }
        Err(TokenError::new(Situation::IncorrectPin, "PIN incorrecto"))
    }
}

#[test]
fn sign_on_token_with_prompter_requires_an_open_cycle() {
    use crate::identity::application::tests::NoToken;

    let session = SigningSession::default();
    let prompter = PreconfiguredSecretPrompter::new("1234");
    let error = sign_on_token_with_prompter(&NoToken, &session, &prompter, Language::Spanish)
        .expect_err("no hay ciclo abierto");
    assert_eq!(error.situation, "unknown");
}

#[test]
fn a_wrong_pin_for_a_batch_is_asked_again_before_the_batch_runs() {
    let prompter = MockSecretPrompter::with_secrets(&["0000", "1234"]);

    let secret = secret_for_the_batch(
        &ATokenThatAcceptsOnly1234,
        &a_certificate("FIRMA", b"der"),
        &prompter,
        Language::Spanish,
        &ProtectedSecret::from_str(""),
    )
    .expect("el segundo PIN es el bueno");

    assert_eq!(secret.expose_secret(), Ok("1234"));
    let asked = prompter.recorded_requests();
    assert_eq!(asked.len(), 2, "el PIN equivocado se vuelve a pedir");
    assert!(!asked[0].incorrect_secret);
    assert!(asked[1].incorrect_secret, "la segunda vez avisa del error");
}

#[test]
fn a_batch_whose_pin_dialog_is_cancelled_is_not_asked_again() {
    let failure = secret_for_the_batch(
        &ATokenThatAcceptsOnly1234,
        &a_certificate("FIRMA", b"der"),
        &PreconfiguredSecretPrompter::cancelling(),
        Language::Spanish,
        &ProtectedSecret::from_str(""),
    )
    .expect_err("cancelar el diálogo no deja secreto");

    assert_eq!(failure.situation, "userCancelled");
}

#[test]
fn a_batch_whose_certificate_needs_no_pin_is_signed_without_asking() {
    use crate::identity::application::tests::NoToken;

    let prompter = MockSecretPrompter::with_secrets(&[]);

    let secret = secret_for_the_batch(
        &NoToken,
        &a_certificate("FIRMA", b"der"),
        &prompter,
        Language::Spanish,
        &ProtectedSecret::from_str(""),
    )
    .expect("sin PIN no hay nada que pedir");

    assert!(secret.is_empty());
    assert!(
        prompter.recorded_requests().is_empty(),
        "no sale el diálogo"
    );
}

#[test]
fn a_pin_typed_in_the_window_closes_the_batch_without_the_dialog() {
    let prompter = MockSecretPrompter::with_secrets(&[]);

    let secret = secret_for_the_batch(
        &ATokenThatAcceptsOnly1234,
        &a_certificate("FIRMA", b"der"),
        &prompter,
        Language::Spanish,
        &ProtectedSecret::from_str("1234"),
    )
    .expect("el PIN tecleado se usa tal cual");

    assert_eq!(secret.expose_secret(), Ok("1234"));
    assert!(
        prompter.recorded_requests().is_empty(),
        "no sale el diálogo"
    );
}

#[test]
fn the_open_cycle_is_signed_with_the_typed_pin_or_through_the_dialog() {
    use crate::identity::application::tests::NoToken;

    let session = SigningSession::default();
    let prompter = MockSecretPrompter::with_secrets(&[]);

    for pin in ["1234", ""] {
        let failure = signed_on_the_token(
            &NoToken,
            &session,
            &prompter,
            Language::Spanish,
            &ProtectedSecret::from_str(pin),
        )
        .expect_err("sin ciclo abierto no hay nada que firmar");
        assert_eq!(failure.situation, "unknown", "con PIN «{pin}»");
    }
}
