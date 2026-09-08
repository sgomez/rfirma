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
fn the_selection_the_published_client_sends_is_what_the_site_wants() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://selectcert?op=selectcert&idsession={CREDENTIAL}"
    )));
    assert!(matches!(request, SiteRequest::SelectCertificate(_)));
}

#[test]
fn an_operation_that_is_not_attended_is_a_request_with_its_refusal() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://countersign?op=countersign&idsession={CREDENTIAL}&format=PAdES"
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
            signature: b"%PDF".to_vec(),
        }),
        "-_-_|JVBERg=="
    );
}

#[test]
fn a_save_the_published_client_sends_is_what_the_site_wants() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://save?op=save&idsession={CREDENTIAL}&dat=JVBERg=="
    )));
    assert!(matches!(request, SiteRequest::Save(_)));
}

#[test]
fn a_sign_and_save_with_dat_is_what_the_site_wants() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA&dat=JVBERg=="
    )));
    assert!(matches!(request, SiteRequest::SignAndSave(_)));
}

#[test]
fn a_sign_and_save_without_dat_is_what_the_site_wants_too() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format=PAdES&\
         algorithm=SHA256withRSA"
    )));
    let SiteRequest::SignAndSave(request) = request else {
        panic!("sin 'dat' el documento queda por elegir, no se rechaza: {request:?}");
    };
    assert_eq!(request.document(), None);
}

#[test]
fn a_load_the_published_client_sends_is_what_the_site_wants() {
    let request = V4Codec.decode(&an_operation(&format!(
        "afirma://load?op=load&idsession={CREDENTIAL}"
    )));
    assert!(matches!(request, SiteRequest::Load(_)));
}

#[test]
fn a_save_goes_out_as_the_literal_the_client_expects() {
    assert_eq!(V4Codec.encode(&SiteOutcome::Saved), "SAVE_OK");
}

#[test]
fn a_load_goes_out_as_name_and_standard_base64_joined_by_a_bar() {
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Loaded(vec![(
            "firma.pdf".to_owned(),
            b"%PDF".to_vec()
        )])),
        "firma.pdf:JVBERg=="
    );
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Loaded(vec![
            ("uno.pdf".to_owned(), b"%PDF".to_vec()),
            ("dos.pdf".to_owned(), b"%PDF".to_vec()),
        ])),
        "uno.pdf:JVBERg==|dos.pdf:JVBERg=="
    );
}

/// Un lote local de un elemento, con el JSON que la sede mete en `dat`.
fn a_local_batch(lote: &str) -> AfirmaUrl {
    an_operation(&format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonbatch=true&\
         localBatchProcess=true&dat={}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(lote)
    ))
}

#[test]
fn a_local_batch_arrives_already_read_and_the_remote_one_does_not() {
    let request = V4Codec.decode(&a_local_batch(
        "{\"algorithm\":\"SHA256\",\"format\":\"CAdES\",\
         \"singlesigns\":[{\"id\":\"001\",\"datareference\":\"AAAA\"}]}",
    ));
    let SiteRequest::LocalBatch(ask) = request else {
        panic!("el lote local llega leido: {request:?}");
    };
    assert_eq!(ask.batch.signs().len(), 1);
    assert_eq!(ask.batch.signs()[0].id(), "001");

    let remote = V4Codec.decode(&an_operation(&format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonbatch=true&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("{\"algorithm\":\"SHA256\",\"singlesigns\":[]}")
    )));
    assert!(matches!(remote, SiteRequest::Batch(_)));
}

#[test]
fn a_local_batch_the_site_wrote_wrong_is_not_attended() {
    let request = V4Codec.decode(&a_local_batch(
        "{\"algorithm\":\"SHA256\",\"singlesigns\":[]}",
    ));
    let SiteRequest::NotAttended(refusal) = request else {
        panic!("un lote local sin formato no se atiende: {request:?}");
    };
    assert!(refusal.answer().on_the_wire().starts_with("SAF_"));
}

#[test]
fn a_batch_without_a_certificate_goes_out_as_plain_base64_of_the_result() {
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Batch {
            result: b"<xml/>".to_vec(),
            signer_der: None,
        }),
        "PHhtbC8+"
    );
}

#[test]
fn a_batch_with_a_certificate_appends_its_standard_base64_behind_a_bar() {
    assert_eq!(
        V4Codec.encode(&SiteOutcome::Batch {
            result: b"<xml/>".to_vec(),
            signer_der: Some(vec![0xfb, 0xff, 0xbf]),
        }),
        "PHhtbC8+|+/+/"
    );
}

#[test]
fn the_cancellation_and_the_refusals_go_out_as_the_catalogue_writes_them() {
    assert_eq!(V4Codec.encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = V4Codec.encode(&SiteOutcome::Refused(SiteRefusal::ScratchUnwritable(
        "detalle que no sale".to_owned(),
    )));
    assert!(refused.starts_with("SAF_"), "{refused}");
    assert!(
        !refused.contains("detalle que no sale"),
        "el detalle no sale"
    );
}
