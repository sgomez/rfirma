use std::sync::Mutex;

use super::in_the_reader::{a_wrong_pin, the_pin, the_signing_certificate, InTheReader};
use super::the_two_versions;
use crate::crossing::Failure;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::{PinWarning, SecretName};
use crate::identity::ports::{
    prompted_until_accepted, PromptedError, SecretPromptError, SecretPromptRequest, SecretPrompter,
    Token,
};
use crate::signing::adapters::prompted_secret::batch_signed_with_one_secret;
use crate::signing::domain::Language;

/// Lo que vio el diálogo al abrirse: el aviso, si era un reintento y cuántas sesiones tenían la tarjeta.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    warning: PinWarning,
    retry: bool,
    open_sessions: usize,
}

/// Quien teclea, en orden, los PIN que se le dan, y apunta lo que ve cada diálogo.
struct Typist {
    reader: InTheReader,
    typed: Mutex<Vec<ProtectedSecret>>,
    seen: Mutex<Vec<Seen>>,
}

impl Typist {
    fn typing(reader: &InTheReader, typed: Vec<ProtectedSecret>) -> Self {
        Self {
            reader: reader.clone(),
            typed: Mutex::new(typed),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<Seen> {
        std::mem::take(&mut *self.seen.lock().expect("sin envenenar"))
    }
}

impl SecretPrompter for Typist {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        self.seen.lock().expect("sin envenenar").push(Seen {
            warning: request.pin_warning,
            retry: request.incorrect_secret,
            open_sessions: self.reader.open_sessions(),
        });
        let mut typed = self.typed.lock().expect("sin envenenar");
        if typed.is_empty() {
            return Err(SecretPromptError::Cancelled);
        }
        Ok(typed.remove(0))
    }
}

fn a_request() -> SecretPromptRequest {
    SecretPromptRequest {
        secret: SecretName::Pin,
        holder: None,
        language: Language::Spanish,
        incorrect_secret: false,
        pin_warning: PinWarning::Quiet,
        origin_window: None,
    }
}

fn is_incorrect(error: &TokenError) -> bool {
    error.situation() == Situation::IncorrectPin
}

fn sign(dnie: &dyn Token, pin: &ProtectedSecret) -> Result<Vec<u8>, TokenError> {
    dnie.sign_with_secret(
        &the_signing_certificate(),
        pin,
        SignatureAlgorithm::Sha256Rsa,
        b"datos",
    )
}

/// Cada elemento de un lote firma en su ciclo, que abre su propio login dentro del del lote.
fn signed_in_its_own_cycle(dnie: &dyn Token, pin: &ProtectedSecret) -> Result<Vec<u8>, TokenError> {
    dnie.hold_one_login(&the_signing_certificate());
    let signed = sign(dnie, pin);
    dnie.release_the_login(&the_signing_certificate());
    signed
}

fn signed_after_the_dialog(
    dnie: &dyn Token,
    typist: &Typist,
) -> Result<(ProtectedSecret, Vec<u8>), PromptedError<TokenError>> {
    prompted_until_accepted(
        typist,
        a_request(),
        || dnie.pin_warning(&the_signing_certificate()),
        |pin| signed_in_its_own_cycle(dnie, pin),
        is_incorrect,
    )
}

fn accepted_after_the_dialog(
    dnie: &dyn Token,
    typist: &Typist,
) -> Result<(ProtectedSecret, ()), PromptedError<TokenError>> {
    prompted_until_accepted(
        typist,
        a_request(),
        || dnie.pin_warning(&the_signing_certificate()),
        |pin| dnie.accepts_the_secret(&the_signing_certificate(), pin),
        is_incorrect,
    )
}

#[test]
fn the_dialog_says_the_attempts_left_and_a_wrong_pin_is_reread_before_the_next_one() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card());
        let typist = Typist::typing(&reader, vec![a_wrong_pin(), the_pin()]);

        let signed = signed_after_the_dialog(&reader.token(), &typist);

        assert!(signed.is_ok(), "{version}");
        assert_eq!(
            typist.seen(),
            vec![
                Seen {
                    warning: PinWarning::AttemptsLeft(3),
                    retry: false,
                    open_sessions: 0
                },
                Seen {
                    warning: PinWarning::AttemptsLeft(2),
                    retry: true,
                    open_sessions: 0
                },
            ],
            "{version}"
        );
    }
}

#[test]
fn with_no_attempts_left_there_is_no_dialog_and_it_ends_as_pin_locked() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card().with_pin_retries(0));
        let typist = Typist::typing(&reader, vec![the_pin()]);

        let ended = signed_after_the_dialog(&reader.token(), &typist);

        assert!(
            matches!(ended, Err(PromptedError::Attempt(ref error)) if error.situation() == Situation::PinLocked),
            "{version}: {ended:?}"
        );
        assert!(typist.seen().is_empty(), "{version}");
        assert_eq!(reader.pins_sent(), 0, "{version}");
    }
}

#[test]
fn attempts_that_change_between_the_dialog_and_the_send_cut_the_cycle_without_sending_the_pin() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card().losing_a_retry_between_phases());
        let typist = Typist::typing(&reader, vec![the_pin(), the_pin()]);

        let ended = signed_after_the_dialog(&reader.token(), &typist);

        assert!(
            matches!(ended, Err(PromptedError::Attempt(_))),
            "{version}: {ended:?}"
        );
        assert_eq!(typist.seen().len(), 1, "{version}");
        assert_eq!(reader.pins_sent(), 0, "{version}");
    }
}

#[test]
fn a_batch_asks_for_the_pin_once_and_every_signature_checks_it_on_the_card() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card());
        let dnie = reader.token();
        let typist = Typist::typing(&reader, vec![the_pin()]);

        dnie.hold_one_login(&the_signing_certificate());
        let (pin, ()) = accepted_after_the_dialog(&dnie, &typist).expect("PIN aceptado");
        let signed: Vec<_> = (0..3).map(|_| sign(&dnie, &pin)).collect();
        dnie.release_the_login(&the_signing_certificate());

        assert!(signed.iter().all(Result::is_ok), "{version}: {signed:?}");
        assert_eq!(typist.seen().len(), 1, "{version}");
        assert_eq!(reader.pins_sent(), 4, "{version}");
    }
}

#[test]
fn a_batch_asks_again_after_a_wrong_pin_with_the_attempts_left_and_signs_with_the_right_one() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card());
        let dnie = reader.token();
        let typist = Typist::typing(&reader, vec![a_wrong_pin(), the_pin()]);
        let certificate = TokenCertificate::new(the_signing_certificate(), Vec::new());

        let signed = batch_signed_with_one_secret(
            &dnie,
            &certificate,
            &typist,
            Language::Spanish,
            &ProtectedSecret::new(b""),
            |pin| {
                for _ in 0..3 {
                    signed_in_its_own_cycle(&dnie, pin).map_err(Failure::from)?;
                }
                Ok(())
            },
        );

        assert!(signed.is_ok(), "{version}: {signed:?}");
        let warnings: Vec<_> = typist
            .seen()
            .into_iter()
            .map(|seen| (seen.warning, seen.retry))
            .collect();
        assert_eq!(
            warnings,
            vec![
                (PinWarning::AttemptsLeft(3), false),
                (PinWarning::AttemptsLeft(2), true)
            ],
            "{version}"
        );
    }
}

#[test]
fn a_batch_stops_at_the_first_failure_without_spending_more_attempts() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card());
        let dnie = reader.token();

        dnie.hold_one_login(&the_signing_certificate());
        let signed: Vec<_> = (0..3)
            .map(|_| signed_in_its_own_cycle(&dnie, &a_wrong_pin()))
            .collect();
        dnie.release_the_login(&the_signing_certificate());

        assert!(
            signed
                .iter()
                .all(|done| done.as_ref().is_err_and(is_incorrect)),
            "{version}: {signed:?}"
        );
        assert_eq!(reader.pins_sent(), 1, "{version}");
        assert_eq!(
            dnie.pin_warning(&the_signing_certificate()),
            Ok(PinWarning::AttemptsLeft(2)),
            "{version}"
        );
    }
}

#[test]
fn a_batch_asks_again_for_a_pin_of_a_length_the_dnie_does_not_admit() {
    let reader = InTheReader::new(rdnie_emulator::EmulatedDnie::dnie_3());
    let dnie = reader.token();
    let typist = Typist::typing(&reader, vec![ProtectedSecret::from_str("1234"), the_pin()]);

    dnie.hold_one_login(&the_signing_certificate());
    let accepted = accepted_after_the_dialog(&dnie, &typist);
    dnie.release_the_login(&the_signing_certificate());

    assert!(accepted.is_ok());
    assert_eq!(typist.seen().len(), 2);
    assert_eq!(reader.pins_sent(), 1);
}
