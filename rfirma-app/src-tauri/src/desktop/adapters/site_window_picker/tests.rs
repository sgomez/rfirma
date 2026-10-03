use std::ffi::OsString;
use std::path::Path;

use super::*;
use crate::desktop::adapters::paths::Platform;
use crate::desktop::adapters::views::TerminalNoCertificateView;
use crate::desktop::ports::NoCertificateToOffer;
use crate::identity::application::tests::a_usable_certificate;

fn a_choice_of(rows: Vec<Choosable>) -> TerminalChoice {
    TerminalChoice::over(
        TerminalChoiceView::of(
            Path::new("/home/ada/contrato.pdf"),
            TerminalStageView::AskingToSign {
                certificates: Vec::new(),
            },
        ),
        rows,
    )
}

fn a_row(handle: &str, label: &str, secret: StoreSecret) -> Choosable {
    (handle.to_owned(), Ok((a_usable_certificate(label), secret)))
}

fn label_of(choice: &WindowChoice) -> Option<String> {
    match choice {
        WindowChoice::Chosen { certificate, .. } => {
            Some(certificate.reference().label().to_owned())
        }
        WindowChoice::Cancelled => None,
    }
}

fn secret_of(choice: &WindowChoice) -> Option<Vec<u8>> {
    match choice {
        WindowChoice::Chosen { secret, .. } => {
            secret.as_ref().map(|secret| secret.as_bytes().to_vec())
        }
        WindowChoice::Cancelled => None,
    }
}

#[test]
fn a_certificate_whose_store_asks_for_a_pin_waits_for_it_before_deciding() {
    let choice = a_choice_of(vec![
        a_row("0", "uno", StoreSecret::NotNeeded),
        a_row("1", "dos", StoreSecret::TypedOnScreen),
    ]);

    let asked = choice.choose("1").expect("se puede elegir");

    assert_eq!(asked, StoreSecret::TypedOnScreen);
    assert!(!choice.is_decided());
    choice
        .hand_the_secret(ProtectedSecret::from("1234"))
        .expect("había un certificado esperando el PIN");
    assert!(choice.is_decided());
    let taken = choice.taken().expect("la ventana se abrió");
    assert_eq!(label_of(&taken), Some("dos".to_owned()));
    assert_eq!(secret_of(&taken), Some(b"1234".to_vec()));
}

#[test]
fn a_certificate_whose_store_needs_no_pin_is_chosen_at_once_without_a_secret() {
    let choice = a_choice_of(vec![a_row("0", "uno", StoreSecret::NotNeeded)]);

    let asked = choice.choose("0").expect("se puede elegir");

    assert_eq!(asked, StoreSecret::NotNeeded);
    assert!(choice.is_decided());
    let taken = choice.taken().expect("la ventana se abrió");
    assert_eq!(label_of(&taken), Some("uno".to_owned()));
    assert_eq!(secret_of(&taken), None);
}

#[test]
fn a_handle_the_window_did_not_offer_or_an_unusable_row_is_refused() {
    let choice = a_choice_of(vec![(
        "0".to_owned(),
        Err(Failure::new("certificateNotFound", "caducado")),
    )]);

    assert!(choice.choose("7").is_err());
    assert!(choice.choose("0").is_err());
    assert!(!choice.is_decided());
}

#[test]
fn a_pin_with_no_certificate_waiting_for_it_is_refused() {
    let choice = a_choice_of(vec![a_row("0", "uno", StoreSecret::TypedOnScreen)]);

    assert!(choice
        .hand_the_secret(ProtectedSecret::from("1234"))
        .is_err());
    assert!(!choice.is_decided());
}

#[test]
fn closing_without_choosing_is_a_cancellation_and_the_first_decision_wins() {
    let unchosen = a_choice_of(Vec::new());
    let cancelled_first = a_choice_of(vec![a_row("0", "uno", StoreSecret::NotNeeded)]);

    cancelled_first.cancel();
    cancelled_first.choose("0").expect("se puede elegir");

    assert!(matches!(unchosen.taken(), Ok(WindowChoice::Cancelled)));
    assert!(matches!(
        cancelled_first.taken(),
        Ok(WindowChoice::Cancelled)
    ));
}

#[test]
fn a_window_that_could_not_open_gives_back_the_reason() {
    let choice = a_choice_of(Vec::new());

    choice.could_not_open("sin WebKit".to_owned());

    assert_eq!(choice.taken().err(), Some("sin WebKit".to_owned()));
}

#[test]
fn the_view_names_the_document_without_its_folder_and_says_why_nothing_is_offered() {
    let view = TerminalChoiceView::of(
        Path::new("/home/ada/contratos/contrato.pdf"),
        TerminalStageView::from(NoCertificateToOffer::Excluded { owned: 3 }),
    );

    assert_eq!(view.document, "contrato.pdf");
    assert_eq!(
        view.stage,
        TerminalStageView::NoCertificate {
            reason: TerminalNoCertificateView::Excluded,
            owned: 3
        }
    );
}

#[test]
fn a_display_is_a_non_empty_wayland_or_x11_variable_on_linux() {
    let environment = |pairs: &'static [(&'static str, &'static str)]| {
        move |name: &str| {
            pairs
                .iter()
                .find(|(variable, _)| *variable == name)
                .map(|(_, value)| OsString::from(value))
        }
    };

    let linux = |pairs| has_a_display(Platform::Linux, environment(pairs));

    assert!(linux(&[("WAYLAND_DISPLAY", "wayland-0")]));
    assert!(linux(&[("DISPLAY", ":0")]));
    assert!(!linux(&[("DISPLAY", "")]));
    assert!(!linux(&[]));
    assert!(has_a_display(Platform::Windows, environment(&[])));
    assert!(has_a_display(Platform::MacOs, environment(&[])));
}
