//! Pruebas de integración del backend contra el módulo PKCS#11 falso, como un almacén de clase tarjeta (ADR-0014).

#![cfg(not(windows))]

use std::path::Path;
use std::sync::Mutex;

use fake_pkcs11::FakeCard;
use rfirma_lib::identity::adapters::pkcs11;
use rfirma_lib::identity::adapters::pkcs11::stores::{candidate_modules_under, CANDIDATE_MODULES};
use rfirma_lib::identity::adapters::pkcs11::RealToken;
use rfirma_lib::identity::adapters::views::CertificateView;
use rfirma_lib::identity::application::certificates::{
    certificates_with_their_chains, listed_rows, ListedCertificates,
};
use rfirma_lib::identity::domain::certificate::{CertificateRef, TokenCertificate};
use rfirma_lib::identity::domain::protected_secret::ProtectedSecret;
use rfirma_lib::identity::domain::secret::PinWarning;
use rfirma_lib::identity::domain::store::{Store, StoreClass};
use rfirma_lib::identity::ports::{
    CertificateMemory, SecretPromptError, SecretPromptRequest, SecretPrompter,
};
use rfirma_lib::memory_error::MemoryError;
use rfirma_lib::signing::adapters::prompted_secret::{
    batch_signed_with_one_secret, secret_for_the_batch,
};
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

const THREE_DOCUMENTS: [&[u8]; 3] = [b"uno", b"dos", b"tres"];

/// Firma un lote remoto de tres documentos con el secreto del lote, sin parar en los fallos: lo que se corte, lo corta el token.
fn the_batch_signed(card: &FakeCard, typist: &Typist, typed: &str) -> Vec<Result<Vec<u8>, String>> {
    let certificate = signing_certificate_of(card);
    let mut signed = Vec::new();
    batch_signed_with_one_secret(
        &RealToken,
        &certificate,
        typist,
        Language::Spanish,
        &ProtectedSecret::from_str(typed),
        |secret| {
            for document in THREE_DOCUMENTS {
                signed.push(
                    signed_for_the_remote_batch(
                        &RealToken,
                        &certificate,
                        secret,
                        "SHA256",
                        document,
                    )
                    .map_err(|refusal| refusal.situation),
                );
            }
            Ok(())
        },
    )
    .expect("el secreto del lote se abre");
    signed
}

#[test]
fn a_batch_is_asked_for_the_pin_once_and_signs_every_document_with_one_login() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    let signed = the_batch_signed(&card, &typist, "");

    assert!(signed.iter().all(Result::is_ok), "{signed:?}");
    assert_eq!(typist.prompts(), 1);
    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
}

#[test]
fn the_first_failure_of_a_batch_cuts_the_cycle_without_more_logins() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[]);

    let signed = the_batch_signed(&card, &typist, "00000000");

    assert_eq!(
        signed,
        vec![Err("incorrectPin".to_owned()); 3],
        "{:?}",
        card.calls()
    );
    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
    assert_eq!(card.tries_left(), 2);
}

#[test]
fn a_card_removed_in_the_middle_of_a_batch_cuts_the_cycle() {
    let rehearsal = FakeCard::new().expect("la tarjeta falsa deberia montarse");
    the_batch_signed(&rehearsal, &Typist::typing(&[FakeCard::PIN]), "");
    let second_signature = rehearsal
        .calls()
        .iter()
        .enumerate()
        .filter(|(_, call)| call.starts_with("C_SignInit"))
        .nth(1)
        .map(|(index, _)| index + 1)
        .expect("el ensayo firma los tres documentos");
    let card = FakeCard::new()
        .and_then(|card| card.removed_from_call(second_signature))
        .expect("la tarjeta falsa deberia montarse");

    let signed = the_batch_signed(&card, &Typist::typing(&[FakeCard::PIN]), "");

    assert!(signed[0].is_ok(), "{signed:?}");
    assert_eq!(
        signed[1..],
        [Err("tokenAbsent".to_owned()), Err("tokenAbsent".to_owned())],
        "{:?}",
        card.calls()
    );
    assert_eq!(card.calls_to("C_Login").len(), 1, "{:?}", card.calls());
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
fn the_batch_adapter_refuses_a_locked_card_as_a_locked_keystore_with_no_attempts_left() {
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

const AUTHENTICATION_CERTIFICATE: &str = "CertAutenticacion";

struct NothingRemembered;

impl CertificateMemory for NothingRemembered {
    fn remembered_certificate(&self) -> Option<CertificateRef> {
        None
    }

    fn remember_the_certificate(&self, _reference: &CertificateRef) -> Result<(), MemoryError> {
        Ok(())
    }

    fn forget_the_certificate(&self) -> Result<(), MemoryError> {
        Ok(())
    }
}

/// Lo que recibe el panel de firma o Preferencias al listar la tarjeta: la etiqueta y los chips de cada fila.
fn the_desktop_rows_of(card: &FakeCard) -> Vec<(String, Vec<String>)> {
    let installed = tempfile::tempdir().expect("el directorio de instalados deberia crearse");
    listed_rows(
        &RealToken,
        &[Store::module(card.module())],
        installed.path(),
        &ListedCertificates::new(),
        &ListedCertificates::new(),
        &NothingRemembered,
        &Default::default(),
    )
    .expect("la tarjeta falsa deberia listarse")
    .into_iter()
    .map(CertificateView::from)
    .map(|row| (row.label, row.stores))
    .collect()
}

/// Las etiquetas de lo que se ofrece a una sede o a la línea de órdenes antes de su filtro.
fn the_site_candidates_of(card: &FakeCard) -> Vec<String> {
    let mut labels: Vec<String> =
        certificates_with_their_chains(&RealToken, &[Store::module(card.module())])
            .expect("la tarjeta falsa deberia listarse")
            .iter()
            .map(|certificate| certificate.reference().label().to_owned())
            .collect();
    labels.sort();
    labels
}

#[test]
fn the_desktop_lists_only_the_signing_certificate_of_the_dnie_with_the_dnie_chip() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    assert_eq!(
        the_desktop_rows_of(&card),
        vec![(SIGNING_CERTIFICATE.to_owned(), vec!["dnie".to_owned()])]
    );
}

#[test]
fn a_site_is_offered_both_certificates_of_the_dnie_and_never_its_authority() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    assert_eq!(
        the_site_candidates_of(&card),
        vec![
            AUTHENTICATION_CERTIFICATE.to_owned(),
            SIGNING_CERTIFICATE.to_owned()
        ]
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

fn interfered_card() -> FakeCard {
    FakeCard::new()
        .and_then(FakeCard::interfered)
        .expect("la tarjeta falsa deberia montarse")
}

#[test]
fn another_program_using_the_card_before_the_login_makes_the_pin_travel_twice_in_two_sessions() {
    let card = interfered_card();
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("la interferencia no es un rechazo");

    assert_eq!(typist.prompts(), 1);
    assert_eq!(card.tries_left(), 3);
    let calls = card.calls();
    let logins: Vec<usize> = calls
        .iter()
        .enumerate()
        .filter(|(_, call)| call.starts_with("C_Login"))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(logins.len(), 2, "{calls:?}");
    assert!(
        calls[logins[0]..logins[1]]
            .iter()
            .any(|call| call.starts_with("C_OpenSession")),
        "{calls:?}"
    );
}

#[test]
fn a_second_failed_login_after_the_interference_is_not_sent_a_third_time() {
    let card = interfered_card();
    let typist = Typist::typing(&["00000000"]);

    the_batch_secret(&card, &typist).expect_err("el PIN era incorrecto");

    assert_eq!(card.calls_to("C_Login").len(), 2, "{:?}", card.calls());
}

#[test]
fn the_dialog_after_an_interference_does_not_say_the_previous_pin_was_wrong() {
    let card = interfered_card();
    let typist = Typist::typing(&[FakeCard::PIN]);

    the_batch_secret(&card, &typist).expect("la interferencia no es un rechazo");

    assert_eq!(typist.prompts(), 1);
    assert_eq!(typist.warnings(), vec![PinWarning::Quiet]);
}

fn context_logins(card: &FakeCard) -> Vec<String> {
    card.calls_to("C_Login")
        .into_iter()
        .filter(|call| call.contains("CKU_CONTEXT_SPECIFIC"))
        .collect()
}

#[test]
fn a_key_that_always_authenticates_gets_a_context_login_per_signature_without_asking_again() {
    let card = FakeCard::new()
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    let typist = Typist::typing(&[FakeCard::PIN]);

    let signed = the_batch_signed(&card, &typist, "");

    assert!(signed.iter().all(Result::is_ok), "{signed:?}");
    assert_eq!(typist.prompts(), 1);
    assert_eq!(context_logins(&card).len(), 3, "{:?}", card.calls());
    assert_eq!(card.calls_to("C_Login").len(), 4, "{:?}", card.calls());
}

#[test]
fn a_failed_context_login_cuts_the_cycle_without_more_logins() {
    let rehearsal = FakeCard::new()
        .and_then(FakeCard::signals_profile)
        .expect("la tarjeta falsa deberia montarse");
    the_batch_signed(&rehearsal, &Typist::typing(&[FakeCard::PIN]), "");
    let second_context_login = rehearsal
        .calls()
        .iter()
        .enumerate()
        .filter(|(_, call)| call.starts_with("C_Login CKU_CONTEXT_SPECIFIC"))
        .nth(1)
        .map(|(index, _)| index + 1)
        .expect("el ensayo hace tres logins de contexto");
    let card = FakeCard::new()
        .and_then(FakeCard::signals_profile)
        .and_then(|card| card.removed_from_call(second_context_login))
        .expect("la tarjeta falsa deberia montarse");

    let signed = the_batch_signed(&card, &Typist::typing(&[FakeCard::PIN]), "");

    assert!(signed[0].is_ok(), "{signed:?}");
    assert!(signed[1..].iter().all(Result::is_err), "{signed:?}");
    assert_eq!(context_logins(&card).len(), 2, "{:?}", card.calls());
    assert_eq!(card.calls_to("C_Login").len(), 3, "{:?}", card.calls());
}

#[test]
fn a_dnie_profile_card_gets_no_context_login() {
    let card = FakeCard::new().expect("la tarjeta falsa deberia montarse");

    the_batch_signed(&card, &Typist::typing(&[FakeCard::PIN]), "");

    assert!(context_logins(&card).is_empty(), "{:?}", card.calls());
}
