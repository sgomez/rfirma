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
use rfirma_lib::identity::domain::secret::PinWarning;
use rfirma_lib::identity::domain::store::{Store, StoreClass};
use rfirma_lib::identity::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};
use rfirma_lib::signing::adapters::prompted_secret::secret_for_the_batch;
use rfirma_lib::signing::domain::Language;
use rfirma_lib::site::adapters::desk::{secret_for_the_remote_batch, signed_for_the_remote_batch};
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

    fn warnings(&self) -> Vec<PinWarning> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|request| request.pin_warning)
            .collect()
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
fn a_dnie_warns_of_the_last_try_only_in_the_dialog_after_two_failures() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&["00000000", "11111111", FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el tercer PIN es el correcto");

    assert_eq!(
        typist.warnings(),
        vec![PinWarning::Quiet, PinWarning::Quiet, PinWarning::FinalTry]
    );
}

#[test]
fn a_card_on_its_final_try_warns_in_the_first_dialog() {
    let card = FakeCard::with_tries_left(1)
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el PIN correcto abre la tarjeta");

    assert_eq!(typist.warnings(), vec![PinWarning::FinalTry]);
}

#[test]
fn a_card_with_a_low_count_warns_softly() {
    let card = FakeCard::with_tries_left(2)
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el PIN correcto abre la tarjeta");

    assert_eq!(typist.warnings(), vec![PinWarning::CountLow]);
}

#[test]
fn a_card_with_no_signals_is_asked_without_a_warning() {
    let card = FakeCard::new()
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el PIN correcto abre la tarjeta");

    assert_eq!(typist.warnings(), vec![PinWarning::Quiet]);
}

#[test]
fn a_rejected_pin_is_never_sent_again() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&["00000000", "11111111", FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("el tercer PIN es el correcto");

    let logins = card.calls_to("C_Login");
    assert_eq!(logins.len(), typist.prompts(), "{logins:?}");
    for (login, typed) in logins.iter().zip(["00000000", "11111111", FakeCard::PIN]) {
        assert!(login.contains(&format!("{typed:?}")), "{logins:?}");
    }
}

#[test]
fn the_try_that_locks_a_dnie_ends_as_pin_locked_without_asking_again() {
    let card = FakeCard::with_tries_left(1).expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&["00000000", FakeCard::PIN]);

    let failure = the_batch_secret(&card, &typist).expect_err("la tarjeta se ha bloqueado");

    assert_eq!(failure.situation, "pinLocked", "{:?}", card.calls());
    assert_eq!(failure.attempts_left, Some(0));
    assert_eq!(typist.prompts(), 1, "se volvió a pedir el PIN");
    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
}

#[test]
fn a_site_hears_a_locked_keystore_when_the_rejected_pin_locks_a_dnie() {
    let card = FakeCard::with_tries_left(1).expect("la tarjeta falsa deberia montarse");

    let refusal = signed_for_the_remote_batch(
        &RealToken,
        &signing_certificate_of(&card),
        &ProtectedSecret::from_str("00000000"),
        "SHA256",
        b"uno",
    )
    .expect_err("el PIN es incorrecto");

    assert_eq!(refusal.code, SafCode::LockedKeystore);
    assert_eq!(refusal.attempts_left, Some(0));
}

#[test]
fn a_site_hears_one_attempt_left_when_a_rejected_pin_leaves_a_dnie_on_its_final_try() {
    let card = FakeCard::with_tries_left(2).expect("la tarjeta falsa deberia montarse");

    let refusal = signed_for_the_remote_batch(
        &RealToken,
        &signing_certificate_of(&card),
        &ProtectedSecret::from_str("00000000"),
        "SHA256",
        b"uno",
    )
    .expect_err("el PIN es incorrecto");

    assert_eq!(refusal.situation, "incorrectPin");
    assert_eq!(refusal.attempts_left, Some(1));
}

#[test]
fn a_site_hears_one_attempt_left_when_signing_fails_on_a_card_on_its_final_try() {
    let card = FakeCard::with_tries_left(1)
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");

    let refusal = signed_for_the_remote_batch(
        &RealToken,
        &signing_certificate_of(&card),
        &ProtectedSecret::from_str("1234"),
        "SHA256",
        b"uno",
    )
    .expect_err("un PIN tan corto no llega a la tarjeta");

    assert_eq!(refusal.attempts_left, Some(1), "{:?}", card.calls());
    assert_eq!(card.tries_left(), 1);
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
