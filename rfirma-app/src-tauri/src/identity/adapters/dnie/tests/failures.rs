use rdnie_emulator::Step;

use super::in_the_reader::{a_wrong_pin, the_pin, the_signing_certificate, InTheReader};
use super::the_two_versions;
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::PinWarning;
use crate::identity::ports::Token;

fn signed_by(dnie: &dyn Token, pin: &ProtectedSecret) -> Result<Vec<u8>, TokenError> {
    dnie.sign_with_secret(
        &the_signing_certificate(),
        pin,
        SignatureAlgorithm::Sha256Rsa,
        b"datos",
    )
}

#[test]
fn a_wrong_pin_is_incorrect_with_one_attempt_fewer() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card());

        let refused = signed_by(&reader.token(), &a_wrong_pin()).expect_err("el PIN es otro");

        assert_eq!(refused.situation(), Situation::IncorrectPin, "{version}");
        assert_eq!(refused.attempts_left(), Some(2), "{version}");
        assert_eq!(
            reader.token().pin_warning(&the_signing_certificate()),
            Ok(PinWarning::AttemptsLeft(2)),
            "{version}"
        );
    }
}

#[test]
fn a_wrong_pin_on_the_last_attempt_ends_as_pin_locked() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card().with_pin_retries(1));

        let refused = signed_by(&reader.token(), &a_wrong_pin()).expect_err("el PIN es otro");

        assert_eq!(refused.situation(), Situation::PinLocked, "{version}");
    }
}

#[test]
fn with_no_attempts_left_no_pin_is_sent_and_it_ends_as_pin_locked() {
    for (version, card) in the_two_versions() {
        let reader = InTheReader::new(card().with_pin_retries(0));

        let refused = signed_by(&reader.token(), &the_pin()).expect_err("el PIN esta bloqueado");

        assert_eq!(refused.situation(), Situation::PinLocked, "{version}");
        assert_eq!(reader.pins_sent(), 0, "{version}");
    }
}

#[test]
fn a_pin_of_a_length_the_dnie_does_not_admit_is_incorrect_without_spending_an_attempt() {
    for (version, card) in the_two_versions() {
        for impossible in ["1234", "un PIN de diecisiete"] {
            let reader = InTheReader::new(card());
            let pin = ProtectedSecret::from_str(impossible);

            let signing = signed_by(&reader.token(), &pin).expect_err("longitud imposible");
            let accepting = reader
                .token()
                .accepts_the_secret(&the_signing_certificate(), &pin)
                .expect_err("longitud imposible");

            assert_eq!(signing.situation(), Situation::IncorrectPin, "{version}");
            assert_eq!(accepting.situation(), Situation::IncorrectPin, "{version}");
            assert_eq!(reader.pins_sent(), 0, "{version}");
            assert_eq!(
                reader.token().pin_warning(&the_signing_certificate()),
                Ok(PinWarning::AttemptsLeft(3)),
                "{version}"
            );
        }
    }
}

#[test]
fn the_card_taken_out_or_reset_at_any_step_ends_as_a_card_that_is_gone() {
    let steps = [
        Step::Opening,
        Step::Certificates,
        Step::PinRetries,
        Step::Verify,
        Step::Signing,
    ];
    for (version, card) in the_two_versions() {
        for step in steps {
            for (strike, struck) in [
                ("retirada", card().removing_card_at(step)),
                ("reiniciada", card().resetting_card_at(step)),
            ] {
                let reader = InTheReader::new(struck);
                let dnie = reader.token();

                let cycle = dnie
                    .pin_warning(&the_signing_certificate())
                    .and_then(|_| signed_by(&dnie, &the_pin()))
                    .expect_err("la tarjeta ya no esta");

                assert_eq!(
                    cycle.situation(),
                    Situation::TokenAbsent,
                    "{version}, {strike} en {step:?}"
                );
            }
        }
    }
}
