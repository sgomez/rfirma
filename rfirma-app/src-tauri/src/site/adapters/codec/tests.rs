use super::*;
use crate::commands::Failure;
use crate::site::domain::protocol::{ChannelMessage, SafCode, WireAnswer};

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

fn an_operation(text: &str) -> AfirmaUrl {
    let ChannelMessage::Operation { url } = ChannelMessage::read(text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn the_selection_the_published_client_sends_is_what_the_site_wants() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    )));
    assert!(matches!(request, SiteRequest::SelectCertificate(_)));
}

#[test]
fn an_operation_that_is_not_attended_is_a_request_with_its_refusal() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://countersign?op=countersign&idsession={CREDENTIAL}"
    )));
    let SiteRequest::NotAttended(refusal) = request else {
        panic!("la contrafirma no se atiende: {request:?}");
    };
    assert!(refusal.answer().on_the_wire().starts_with("SAF_04"));
}

#[test]
fn a_certificate_goes_out_as_url_safe_base64_and_nothing_else() {
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Certificate(vec![0xfb, 0xff, 0xbf])),
        "-_-_"
    );
}

#[test]
fn a_signature_goes_out_behind_its_certificate_separated_by_a_bar() {
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Signature {
            signer_der: vec![0xfb, 0xff, 0xbf],
            signed: b"%PDF".to_vec(),
        }),
        "-_-_|JVBERg=="
    );
}

#[test]
fn the_cancellation_and_the_refusals_go_out_as_the_catalogue_writes_them() {
    assert_eq!(V4Codec.encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = V4Codec.encode(&SiteOutcome::Refused {
        answer: WireAnswer::refused(SafCode::NoCertificatesInKeystore),
        failure: Failure::new("certificateNotFound", "detalle que no sale"),
    });
    assert!(refused.starts_with("SAF_19"), "{refused}");
    assert!(
        !refused.contains("detalle que no sale"),
        "el detalle no sale"
    );
}
