use super::*;
use crate::site::application::errand::SiteRefusal;
use crate::site::domain::protocol::ChannelMessage;

const CREDENTIAL: &str = "8jAkPZfRw2mQxN4TbYuL";

fn an_operation(text: &str) -> AfirmaUrl {
    let ChannelMessage::Operation { url } = ChannelMessage::read(text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn it_decodes_exactly_like_the_fourth_protocol() {
    let message = an_operation(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    ));
    assert_eq!(
        format!("{:?}", V1Codec::new(1).decode(&message)),
        format!("{:?}", V4Codec.decode(&message))
    );
}

#[test]
fn it_encodes_a_signature_exactly_like_the_fourth_protocol() {
    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
        chosen_document: None,
    };
    assert_eq!(V1Codec::new(1).encode(&outcome), V4Codec.encode(&outcome));
    assert_eq!(V1Codec::new(1).encode(&outcome), "-_-_|JVBERg==");
}

fn a_signature_over_a_chosen_document() -> SiteOutcome {
    SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
        chosen_document: Some("documento.txt".to_owned()),
    }
}

#[test]
fn below_the_third_protocol_it_leaves_the_chosen_document_out_of_a_signature() {
    for version in 1..=2 {
        assert_eq!(
            V1Codec::new(version).encode(&a_signature_over_a_chosen_document()),
            "-_-_|JVBERg==",
            "v={version}"
        );
    }
}

#[test]
fn with_the_third_protocol_it_adds_the_chosen_document_like_the_fourth() {
    let outcome = a_signature_over_a_chosen_document();

    assert_eq!(V1Codec::new(3).encode(&outcome), V4Codec.encode(&outcome));
    assert_eq!(
        V1Codec::new(3).encode(&outcome),
        "-_-_|JVBERg==|eyJmaWxlbmFtZSI6ICJkb2N1bWVudG8udHh0In0="
    );
}

#[test]
fn it_encodes_a_batch_exactly_like_the_fourth_protocol() {
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };
    assert_eq!(V1Codec::new(1).encode(&outcome), V4Codec.encode(&outcome));
    assert_eq!(V1Codec::new(1).encode(&outcome), "PHhtbC8+|+/+/");
}

#[test]
fn it_encodes_the_cancellation_and_refusals_from_the_same_closed_catalogue() {
    assert_eq!(V1Codec::new(1).encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = V1Codec::new(1).encode(&SiteOutcome::Refused(SiteRefusal::ScratchUnwritable(
        "detalle que no sale".to_owned(),
    )));
    assert!(refused.starts_with("SAF_"), "{refused}");
}
