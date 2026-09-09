use super::*;
use crate::identity::domain::certificate::{CertificateStatus, ListedCertificate};
use crate::identity::domain::store::StoreClass;
use crate::signing::domain::bridge::Format;

fn asking_with(label: &str) -> Moment {
    Moment::AskingForConsent {
        certificates: vec![ListedCertificate {
            id: "cert-1".to_owned(),
            label: label.to_owned(),
            holder_name: String::new(),
            id_number: String::new(),
            issuer: String::new(),
            store: StoreClass::Card,
            status: CertificateStatus::Valid { not_after: 0 },
            remembered: false,
        }],
    }
}

fn a_pending_signature() -> PendingSignature {
    PendingSignature {
        document: "doc-1".to_owned(),
        filter: SiteFilter::default(),
        format: Format::Pades,
        algorithm: AskedAlgorithm::Sha256,
        operation: crate::signing::domain::bridge::SignatureOperation::Sign,
        from_the_site: BTreeMap::new(),
        unregistered_signatures: false,
        saving: None,
    }
}

#[test]
fn the_moment_survives_a_window_that_was_not_listening_yet() {
    let live = LiveErrand::default();
    assert!(live.moment().is_none(), "sin trámite no hay momento");

    live.note(Moment::Waiting);
    assert_eq!(live.moment(), Some(Moment::Waiting));
}

#[test]
fn reading_the_moment_leaves_it_where_it_was() {
    let live = LiveErrand::default();
    live.note(Moment::Waiting);

    let _ = live.moment();
    assert_eq!(live.moment(), Some(Moment::Waiting));
}

#[test]
fn the_last_moment_is_the_one_that_is_kept() {
    let live = LiveErrand::default();
    live.note(Moment::Waiting);
    live.note(asking_with("FIRMA"));

    assert_eq!(live.moment(), Some(asking_with("FIRMA")));
}

#[test]
fn a_consented_signature_is_never_an_identity_to_hand_over() {
    let live = LiveErrand::default();
    live.remember_signature(a_pending_signature());

    assert!(live.what_the_site_asked().is_none());
    assert!(live.the_signature_consented().is_some());
}

#[test]
fn a_consented_identity_is_never_a_signature_to_begin() {
    let live = LiveErrand::default();
    live.remember_identity(SiteFilter::default(), false);

    assert!(live.what_the_site_asked().is_some());
    assert!(live.the_signature_consented().is_none());
}

#[test]
fn ending_leaves_nothing_to_answer_with() {
    let live = LiveErrand::default();
    live.remember_signature(a_pending_signature());
    live.end();
    assert!(live.the_signature_consented().is_none());

    live.remember_identity(SiteFilter::default(), false);
    live.end();
    assert!(live.what_the_site_asked().is_none());
}

#[test]
fn operation_moments_are_posterior_to_waiting() {
    let waiting = Moment::Waiting;
    let asking = asking_with("FIRMA");
    let no_cert = Moment::NoCertificate {
        reason: crate::site::application::errand::NoCertificate::NotOne,
        owned: 0,
    };
    let dead_end = Moment::NoChannel(crate::site::application::errand::NoChannel::LocalCaMissing);

    assert!(asking.is_posterior_to(&waiting));
    assert!(no_cert.is_posterior_to(&waiting));
    assert!(dead_end.is_posterior_to(&waiting));
    assert!(!waiting.is_posterior_to(&waiting));
    assert!(!waiting.is_posterior_to(&asking));
    assert!(!asking.is_posterior_to(&dead_end));
}
