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
        format!("{:?}", V3Codec.decode(&message)),
        format!("{:?}", V4Codec.decode(&message))
    );
}

#[test]
fn it_encodes_a_signature_exactly_like_the_fourth_protocol() {
    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signed: b"%PDF".to_vec(),
    };
    assert_eq!(V3Codec.encode(&outcome), V4Codec.encode(&outcome));
    assert_eq!(V3Codec.encode(&outcome), "-_-_|JVBERg==");
}

#[test]
fn it_encodes_a_batch_exactly_like_the_fourth_protocol() {
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };
    assert_eq!(V3Codec.encode(&outcome), V4Codec.encode(&outcome));
    assert_eq!(V3Codec.encode(&outcome), "PHhtbC8+|+/+/");
}

#[test]
fn it_encodes_the_cancellation_and_refusals_from_the_same_closed_catalogue() {
    assert_eq!(V3Codec.encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = V3Codec.encode(&SiteOutcome::Refused(SiteRefusal::ScratchUnwritable(
        "detalle que no sale".to_owned(),
    )));
    assert!(refused.starts_with("SAF_"), "{refused}");
}
