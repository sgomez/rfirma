use super::*;
use crate::identity::domain::certificate::{CertificateStatus, ListedCertificate};
use crate::identity::domain::store::StoreClass;
use crate::signing::domain::bridge::Format;
use crate::site::application::errand::SiteRequest;
use crate::site::domain::protocol::{AskedAlgorithm, SiteFilter};
use std::collections::BTreeMap;

struct NullCodec;

impl ProtocolCodec for NullCodec {
    fn decode(&self, _message: &AfirmaUrl) -> SiteRequest {
        unimplemented!("esta prueba no decodifica ningún mensaje")
    }

    fn encode(&self, _outcome: &SiteOutcome) -> String {
        String::new()
    }
}

fn asking_with(label: &str) -> Moment {
    Moment::AskingForConsent {
        certificates: vec![ListedCertificate {
            id: "cert-1".to_owned(),
            label: label.to_owned(),
            holder_name: String::new(),
            stamped_signer: String::new(),
            given_name: String::new(),
            surname: String::new(),
            id_number: String::new(),
            organization_identifier: None,
            entity_name: None,
            issuer: String::new(),
            certificate_serial_number: String::new(),
            store: StoreClass::Card,
            stores: vec![StoreClass::Card],
            status: CertificateStatus::Valid { not_after: 0 },
            remembered: false,
        }],
    }
}

fn a_pending_signature() -> PendingSignature {
    PendingSignature {
        document: "doc-1".to_owned(),
        sha1_of_the_data: None,
        filter: SiteFilter::default(),
        format: Format::Pades,
        algorithm: AskedAlgorithm::Sha256,
        operation: crate::signing::domain::bridge::SignatureOperation::Sign,
        from_the_site: BTreeMap::new(),
        unregistered_signatures: false,
        headless: false,
        saving: None,
        through_the_server: None,
        area: None,
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
fn before_any_operation_the_origin_is_absent() {
    let live = LiveErrand::default();

    assert_eq!(live.origin(), SiteOrigin::absent());
}

#[test]
fn a_noted_origin_replaces_the_one_of_the_previous_operation() {
    let live = LiveErrand::default();

    live.note_origin(SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es")));
    assert_eq!(
        live.origin().host(),
        Some("sede.ejemplo.gob.es"),
        "el origen que llegó con la operación queda apuntado"
    );

    live.note_origin(SiteOrigin::absent());
    assert_eq!(
        live.origin(),
        SiteOrigin::absent(),
        "la operación siguiente sustituye al origen de la anterior"
    );
}

#[test]
fn beginning_a_new_errand_clears_the_origin_of_the_previous_one() {
    let live = LiveErrand::default();
    live.note_origin(SiteOrigin::from_header(Some("https://sede.ejemplo.gob.es")));

    let began = live.begin(Errand::of(
        NegotiatedCredential::Absent,
        ArrivalMode::Awaited,
        Arc::new(NullCodec),
    ));

    assert!(began);
    assert_eq!(
        live.origin(),
        SiteOrigin::absent(),
        "un trámite nuevo no hereda el origen del trámite anterior"
    );
}
