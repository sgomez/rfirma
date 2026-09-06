use super::*;
use crate::identity::adapters::views::CertificateView;
use crate::signing::adapters::views::StatusView;

fn asking_with(label: &str) -> Moment {
    Moment::AskingForConsent {
        certificates: vec![CertificateView {
            id: "cert-1".to_owned(),
            label: label.to_owned(),
            holder_name: String::new(),
            id_number: String::new(),
            issuer: String::new(),
            store: "card".to_owned(),
            status: StatusView::Valid { not_after: 0 },
            remembered: false,
        }],
    }
}

fn a_pending_signature() -> PendingSignature {
    PendingSignature {
        document: "doc-1".to_owned(),
        filter: SiteFilter::default(),
        from_the_site: BTreeMap::new(),
        unregistered_signatures: false,
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
    live.remember_identity(SiteFilter::default());

    assert!(live.what_the_site_asked().is_some());
    assert!(live.the_signature_consented().is_none());
}

#[test]
fn ending_leaves_nothing_to_answer_with() {
    let live = LiveErrand::default();
    live.remember_signature(a_pending_signature());
    live.end();
    assert!(live.the_signature_consented().is_none());

    live.remember_identity(SiteFilter::default());
    live.end();
    assert!(live.what_the_site_asked().is_none());
}
