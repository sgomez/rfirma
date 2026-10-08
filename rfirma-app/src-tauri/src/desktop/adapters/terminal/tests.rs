use super::prompt_for;
use crate::desktop::ports::AskedSecret;
use crate::identity::domain::secret::{PinWarning, SecretName};

fn asked(warning: PinWarning) -> AskedSecret<'static> {
    AskedSecret {
        name: SecretName::Pin,
        alias: "CertFirmaDigital",
        incorrect: false,
        warning,
    }
}

#[test]
fn the_terminal_asks_for_the_pin_with_no_warning_when_the_card_gives_none() {
    assert_eq!(
        prompt_for(&asked(PinWarning::Quiet)),
        "PIN de «CertFirmaDigital»: "
    );
}

#[test]
fn the_terminal_warns_of_the_final_try_before_asking_for_the_pin() {
    assert_eq!(
        prompt_for(&asked(PinWarning::FinalTry)),
        "rfirma: último intento: si el PIN no es correcto, la tarjeta se bloqueará.\n\
         PIN de «CertFirmaDigital»: "
    );
}

#[test]
fn the_terminal_warns_softly_of_earlier_failed_tries() {
    assert_eq!(
        prompt_for(&asked(PinWarning::CountLow)),
        "rfirma: ya ha habido algún intento fallido con esta tarjeta.\n\
         PIN de «CertFirmaDigital»: "
    );
}

#[test]
fn after_a_wrong_pin_the_terminal_says_so_before_the_warning() {
    let mut retry = asked(PinWarning::FinalTry);
    retry.incorrect = true;

    assert_eq!(
        prompt_for(&retry),
        "rfirma: no es correcto; vuelve a intentarlo.\n\
         rfirma: último intento: si el PIN no es correcto, la tarjeta se bloqueará.\n\
         PIN de «CertFirmaDigital»: "
    );
}
