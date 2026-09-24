use super::*;
use crate::site::application::errand::SiteRefusal;
use crate::site::domain::protocol::{ChannelMessage, SafCode};

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
        signature: b"%PDF".to_vec(),
        chosen_document: None,
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

const DISPATCHED_VERBS: [&str; 8] = [
    "selectcert",
    "sign",
    "cosign",
    "countersign",
    "signandsave",
    "save",
    "load",
    "batch",
];

fn code_of(request: &SiteRequest) -> Option<SafCode> {
    match request {
        SiteRequest::NotAttended(refusal) => Some(refusal.code()),
        _ => None,
    }
}

fn an_operation_declaring(verb: &str, version: &str) -> AfirmaUrl {
    an_operation(&format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format=NotAFormat&ver={version}"
    ))
}

#[test]
fn an_operation_asking_for_a_later_version_is_refused_with_saf_21() {
    for verb in DISPATCHED_VERBS {
        let request = V3Codec.decode(&an_operation_declaring(verb, "5"));

        assert_eq!(
            code_of(&request),
            Some(SafCode::UnsupportedProcedure),
            "con {verb}: {request:?}"
        );
    }
}

#[test]
fn an_operation_asking_for_the_fourth_version_or_an_earlier_one_is_not_refused_for_it() {
    for version in ["4", "3", "1", "-1", "x"] {
        for verb in DISPATCHED_VERBS {
            let request = V3Codec.decode(&an_operation_declaring(verb, version));

            assert_ne!(
                code_of(&request),
                Some(SafCode::UnsupportedProcedure),
                "con {verb} y ver={version}: {request:?}"
            );
        }
    }
    assert_eq!(
        code_of(&V3Codec.decode(&an_operation_declaring("sign", "4"))),
        Some(SafCode::UnsupportedFormat)
    );
}

#[test]
fn a_parameter_the_original_parses_first_wins_over_the_later_version() {
    let message = an_operation(&format!(
        "afirma://sign?op=sign&idsession={CREDENTIAL}&key=short&ver=5"
    ));

    assert_eq!(code_of(&V3Codec.decode(&message)), Some(SafCode::Params));
}

#[test]
fn an_operation_that_is_not_dispatched_is_refused_as_such_whatever_its_version() {
    let message = an_operation(&format!(
        "afirma://unknown?op=unknown&idsession={CREDENTIAL}&ver=5"
    ));

    assert_eq!(
        code_of(&V3Codec.decode(&message)),
        Some(SafCode::UnsupportedOperation)
    );
}

#[test]
fn the_fourth_protocol_still_ignores_the_version_of_the_operation() {
    for verb in DISPATCHED_VERBS {
        let request = V4Codec.decode(&an_operation_declaring(verb, "5"));

        assert_ne!(
            code_of(&request),
            Some(SafCode::UnsupportedProcedure),
            "con {verb}: {request:?}"
        );
    }
}
