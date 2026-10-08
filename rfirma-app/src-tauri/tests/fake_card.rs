//! Pruebas de integración del backend contra el módulo PKCS#11 falso, como un almacén de clase tarjeta (ADR-0014).

#![cfg(not(windows))]

use std::path::Path;
use std::sync::Mutex;

use fake_pkcs11::FakeCard;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::adapters::pkcs11::stores::{candidate_modules_under, CANDIDATE_MODULES};
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::domain::certificate::TokenCertificate;
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::store::{Store, StoreClass};
use rfirma_lib::identity::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};
use rfirma_lib::signing::adapters::prompted_secret::secret_for_the_batch;
use rfirma_lib::signing::domain::Language;
use rfirma_lib::site::adapters::desk::secret_for_the_remote_batch;
use rfirma_lib::site::domain::protocol::SafCode;

const SIGNING_CERTIFICATE: &str = "CertFirmaDigital";

/// La persona ante el diálogo del PIN: teclea, por orden, lo que se le dio, y apunta cada petición.
struct Typist {
    typed: Mutex<Vec<&'static str>>,
    requests: Mutex<Vec<SecretPromptRequest>>,
}

impl Typist {
    fn typing(typed: &[&'static str]) -> Self {
        Self {
            typed: Mutex::new(typed.iter().rev().copied().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn prompts(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl SecretPrompter for Typist {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        self.requests.lock().unwrap().push(request.clone());
        self.typed
            .lock()
            .unwrap()
            .pop()
            .map(ProtectedSecret::from_str)
            .ok_or(SecretPromptError::Cancelled)
    }
}

fn signing_certificate_of(card: &FakeCard) -> TokenCertificate {
    pkcs11::list_certificates_across(&[Store::module(card.module())])
        .expect("la tarjeta falsa debería listarse")
        .into_iter()
        .find(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE)
        .expect("la tarjeta falsa lleva el certificado de firma")
}

fn the_batch_secret(
    card: &FakeCard,
    typist: &Typist,
) -> Result<ProtectedSecret, rfirma_lib::crossing::Failure> {
    secret_for_the_batch(
        &RealToken,
        &signing_certificate_of(card),
        typist,
        Language::Spanish,
        &ProtectedSecret::new(b""),
    )
}

#[test]
fn a_card_that_declares_its_pin_locked_receives_no_login_and_is_not_asked_for_the_pin() {
    let card = FakeCard::locked()
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    let failure = the_batch_secret(&card, &typist).expect_err("una tarjeta bloqueada no firma");

    assert_eq!(failure.situation, "pinLocked");
    assert_eq!(failure.attempts_left, Some(0));
    assert_eq!(
        typist.prompts(),
        0,
        "se pidió el PIN de una tarjeta bloqueada"
    );
    assert!(
        card.calls_to("C_Login").is_empty(),
        "{:?}",
        card.calls_to("C_Login")
    );
}

#[test]
fn a_site_hears_a_locked_keystore_with_no_attempts_left() {
    let card = FakeCard::locked()
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");

    let refusal = secret_for_the_remote_batch(&RealToken, &signing_certificate_of(&card))
        .expect_err("una tarjeta bloqueada no firma");

    assert_eq!(refusal.code, SafCode::LockedKeystore);
    assert_eq!(refusal.situation, "pinLocked");
    assert_eq!(refusal.attempts_left, Some(0));
    assert!(card.calls_to("C_Login").is_empty(), "{:?}", card.calls());
}

#[test]
fn a_card_locked_by_the_rejected_try_is_not_sent_the_pin_again() {
    let card = FakeCard::with_tries_left(1)
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&["00000000", FakeCard::PIN]);

    let failure = the_batch_secret(&card, &typist).expect_err("la tarjeta se ha bloqueado");

    assert_eq!(
        failure.situation,
        "pinLocked",
        "{failure:?} {:?}",
        card.calls()
    );
    assert_eq!(
        card.calls_to("C_Login").len(),
        1,
        "{:?}",
        card.calls_to("C_Login")
    );
}

#[test]
fn a_dnie_with_no_failed_tries_is_asked_for_the_pin_as_before() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el PIN correcto abre la tarjeta");

    assert_eq!(typist.prompts(), 1);
    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
}

#[test]
fn the_fake_module_is_a_card_store_that_lists_without_asking_for_the_pin() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let store = Store::module(card.module());
    assert_eq!(store.class(), StoreClass::Card);

    let listed = pkcs11::list_certificates_across(&[store]).expect("listar no debería fallar");

    assert!(
        listed
            .iter()
            .any(|certificate| certificate.reference().label() == SIGNING_CERTIFICATE),
        "el certificado de firma tenía que salir sin sesión"
    );
    assert!(
        card.calls_to("C_Login").is_empty(),
        "listar pidió el secreto a la tarjeta: {:?}",
        card.calls()
    );
}

#[test]
fn the_fake_module_is_not_among_the_modules_rfirma_looks_for() {
    let candidates: Vec<String> = candidate_modules_under(Path::new("/usr/lib"))
        .iter()
        .map(|path| path.display().to_string())
        .chain(CANDIDATE_MODULES.iter().map(|path| (*path).to_owned()))
        .collect();

    assert!(
        candidates.iter().all(|path| !path.contains("fake")),
        "{candidates:?}"
    );
}
