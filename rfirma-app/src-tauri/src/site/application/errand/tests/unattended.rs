//! Pruebas de cuándo la compilación de conformidad consiente sola: solo si no queda nada que decidir. Solo en pruebas.

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::secret::StoreSecret;
use crate::signing::application::session;
use crate::signing::application::tests::a_memory;
use crate::signing::application::tests::A_CADES_SIGNATURE;
use crate::site::application::errand::outcome::SavingHints;
use crate::site::application::errand::unattended::{
    consent_unattended, nothing_to_decide, nothing_to_decide_on, switched_on, Unattended,
};
use crate::site::application::errand::*;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    IfCancelled, NegotiatedCredential, SiteFilter, SiteVisibleSignature,
};
use base64::Engine as _;
use std::ffi::OsStr;

fn no_secret(_: &str) -> Option<StoreSecret> {
    Some(StoreSecret::NotNeeded)
}

fn never_asked(_: &str) -> Option<StoreSecret> {
    panic!("una identificacion no abre el almacen (ADR-0025)")
}

fn a_signature(change: impl FnOnce(&mut SigningConsent)) -> ErrandStep {
    let ErrandStep::AskingToSign(mut consent) = a_consent_to_sign("") else {
        panic!("una firma de sede pide consentimiento");
    };
    change(&mut consent);
    ErrandStep::AskingToSign(consent)
}

fn a_second_certificate(consent: &mut SigningConsent) {
    let mut other = consent.certificates[0].clone();
    other.id = "otro".to_owned();
    consent.certificates.push(other);
}

fn the_only_handle(step: &ErrandStep) -> String {
    let ErrandStep::AskingToSign(consent) = step else {
        panic!("esperaba una firma: {step:?}");
    };
    consent.certificates[0].id.clone()
}

fn an_identification(how_many: usize) -> ErrandStep {
    let ErrandStep::AskingToSign(consent) = a_consent_to_sign("") else {
        panic!("una firma de sede pide consentimiento");
    };
    let certificates = (0..how_many)
        .map(|index| {
            let mut row = consent.certificates[0].clone();
            row.id = format!("fila-{index}");
            row
        })
        .collect();
    ErrandStep::AskingForConsent {
        certificates,
        filter: SiteFilter::default(),
        sticky: false,
    }
}

#[test]
fn a_signature_with_one_certificate_and_no_secret_goes_unattended() {
    let step = a_signature(|_| {});

    assert_eq!(
        nothing_to_decide(&step, no_secret),
        Some(Unattended::Sign(the_only_handle(&step)))
    );
}

#[test]
fn a_signature_with_two_certificates_waits_for_the_person() {
    let step = a_signature(a_second_certificate);

    assert_eq!(nothing_to_decide(&step, no_secret), None);
}

#[test]
fn a_signature_whose_store_asks_for_a_pin_waits_for_the_person() {
    let step = a_signature(|_| {});

    for secret in [
        StoreSecret::TypedOnScreen,
        StoreSecret::TypedOnTheReaderKeypad,
    ] {
        assert_eq!(
            nothing_to_decide(&step, |_| Some(secret)),
            None,
            "{secret:?}"
        );
    }
}

#[test]
fn a_signature_whose_secret_cannot_be_read_waits_for_the_person() {
    let step = a_signature(|_| {});

    assert_eq!(nothing_to_decide(&step, |_| None), None);
}

#[test]
fn a_signature_with_an_area_to_mark_waits_for_the_person() {
    for if_cancelled in [
        IfCancelled::Refuses,
        IfCancelled::SignsWhereTheSiteSaid,
        IfCancelled::SignsInvisible,
    ] {
        let step = a_signature(|consent| {
            consent.visible = SiteVisibleSignature::MarkedByThePerson(if_cancelled);
        });

        assert_eq!(
            nothing_to_decide(&step, no_secret),
            None,
            "{if_cancelled:?}"
        );
    }
}

#[test]
fn a_signature_over_unregistered_signatures_waits_for_the_person() {
    let step = a_signature(|consent| consent.unregistered_signatures = true);

    assert_eq!(nothing_to_decide(&step, no_secret), None);
}

#[test]
fn a_signature_that_ends_in_a_save_dialog_waits_for_the_person() {
    let step = a_signature(|consent| {
        consent.saving = Some(Box::new(SavingHints {
            filename: "firma.pdf".to_owned(),
            extensions: vec!["pdf".to_owned()],
            description: None,
            starting_folder: None,
        }));
    });

    assert_eq!(nothing_to_decide(&step, no_secret), None);
}

#[test]
fn a_signature_where_the_site_placed_the_box_goes_unattended() {
    let step = a_signature(|consent| consent.visible = SiteVisibleSignature::PlacedByTheSite);

    assert!(nothing_to_decide(&step, no_secret).is_some());
}

#[test]
fn an_identification_with_one_certificate_goes_unattended_without_opening_the_store() {
    assert_eq!(
        nothing_to_decide(&an_identification(1), never_asked),
        Some(Unattended::Identify("fila-0".to_owned()))
    );
}

#[test]
fn an_identification_with_two_certificates_waits_for_the_person() {
    assert_eq!(nothing_to_decide(&an_identification(2), never_asked), None);
}

#[test]
fn a_local_batch_with_one_certificate_and_no_secret_goes_unattended() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );
    let url = a_local_batch("");

    let step = attend_operation(&desk, &url, decoded(&url), &a_live());

    assert!(
        matches!(
            nothing_to_decide(&step, no_secret),
            Some(Unattended::SignTheBatch(_))
        ),
        "{step:?}"
    );
}

#[test]
fn the_desk_asks_the_store_of_the_candidate_and_a_pin_makes_it_wait() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );
    let url = a_local_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &a_live());

    assert_eq!(nothing_to_decide_on(&desk, &step), None);
    assert_eq!(desk.neighbours.token.secrets_asked(), 1);
}

#[test]
fn what_is_not_a_consent_never_goes_unattended() {
    let steps = [
        ErrandStep::Loading(Box::new(LoadingConsent {
            title: None,
            filename: None,
            extensions: Vec::new(),
            description: None,
            starting_folder: None,
            multiple: false,
            to_sign: None,
        })),
        ErrandStep::NoCertificate {
            reason: NoCertificate::NotOne,
            owned: 0,
            answered: None,
        },
        ErrandStep::Answering(SiteOutcome::Cancelled),
    ];

    for step in steps {
        assert_eq!(nothing_to_decide(&step, no_secret), None, "{step:?}");
    }
}

#[test]
fn only_a_one_turns_the_switch_on() {
    assert!(switched_on(Some(OsStr::new("1"))));
    for value in [None, Some(""), Some("0"), Some("true"), Some("yes")] {
        assert!(!switched_on(value.map(OsStr::new)), "{value:?}");
    }
}

type TestDesk<'a> = ErrandDesk<'a, AnEngine, APolicyEngine, TheNeighbours<'a>>;

/// Una firma de sede consentida sin nadie delante: lo que recibe la sede y si el trámite se acabó.
fn an_unattended_signature(
    signed: impl FnOnce(&TestDesk<'_>, &LiveErrand) -> bool,
) -> (Option<String>, bool) {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    desk.neighbours.ours = ours.clone();
    desk.neighbours.bridge = TheBridge::answering();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, mut wire) = the_wire();
    let step = attend(
        &desk,
        a_signature_asking_for("CAdES", b"hola"),
        handle,
        &live,
    )
    .expect("hay codec negociado");
    let unattended = nothing_to_decide(&step, no_secret).expect("no queda nada que decidir");

    consent_unattended(&desk, &live, &unattended, || signed(&desk, &live));

    let received = what_the_site_received(&mut wire);
    if let Some(line) = &received {
        let encode = base64::engine::general_purpose::URL_SAFE;
        let expected = format!(
            "{}|{}",
            encode.encode(ours[0].der()),
            encode.encode(A_CADES_SIGNATURE)
        );
        assert_eq!(*line, expected, "la misma linea que tras el clic");
    }
    (received, live.current().is_none())
}

#[test]
fn an_unattended_signature_reaches_the_site_by_the_path_of_the_click() {
    let (received, ended) = an_unattended_signature(|desk, _| {
        session::sign_on_token(&desk.neighbours.signer, &desk.neighbours.session, "").is_ok()
    });

    assert!(received.is_some(), "la sede recibe la firma");
    assert!(ended, "contestada la sede, se acabo");
}

#[test]
fn an_unattended_signature_that_does_not_sign_hands_nothing_over() {
    let (received, ended) = an_unattended_signature(|_, _| false);

    assert_eq!(received, None);
    assert!(
        !ended,
        "lo contesta el cierre de la ventana, como tras el clic"
    );
}
