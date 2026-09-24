use super::*;
use crate::site::application::errand::SiteRefusal;
use crate::site::domain::protocol::{decrypt, ChannelMessage};

const KEY: &[u8; 8] = b"12345678";

fn a_key() -> CipherKey {
    CipherKey::from_url_parameter(std::str::from_utf8(KEY).expect("ascii"))
        .expect("la clave tiene la longitud correcta")
        .expect("un valor no vacio siempre produce una clave")
}

fn an_operation(text: &str) -> AfirmaUrl {
    let ChannelMessage::Operation { url } = ChannelMessage::read(text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn it_decodes_exactly_like_the_fourth_protocol() {
    let message = an_operation("afirma://selectcert?op=selectcert");
    let codec = RelayCodec::new(None, 1);

    assert_eq!(
        format!("{:?}", codec.decode(&message)),
        format!(
            "{:?}",
            crate::site::adapters::codec::V4Codec.decode(&message)
        )
    );
}

#[test]
fn without_a_key_the_response_travels_in_plain_base64() {
    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
        chosen_document: None,
    };

    assert_eq!(RelayCodec::new(None, 1).encode(&outcome), "+/+/|JVBERg==");
}

#[test]
fn with_a_key_each_field_is_ciphered_on_its_own_and_recoverable() {
    let key = a_key();
    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
        chosen_document: None,
    };

    let wire = RelayCodec::new(Some(key.clone()), 1).encode(&outcome);
    let (signer, signed) = wire.split_once(RESULT_SEPARATOR).expect("dos campos");

    assert_eq!(
        decrypt(signer, Some(&key)).expect("descifra"),
        vec![0xfb, 0xff, 0xbf]
    );
    assert_eq!(decrypt(signed, Some(&key)).expect("descifra"), b"%PDF");
}

fn a_signature_over_a_chosen_document() -> SiteOutcome {
    SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signature: b"%PDF".to_vec(),
        chosen_document: Some("documento.txt".to_owned()),
    }
}

#[test]
fn below_the_third_protocol_the_chosen_document_stays_out() {
    for version in 1..=2 {
        assert_eq!(
            RelayCodec::new(None, version).encode(&a_signature_over_a_chosen_document()),
            "+/+/|JVBERg==",
            "ver={version}"
        );
    }
}

#[test]
fn from_the_third_protocol_the_chosen_document_travels_as_a_third_field() {
    for version in 3..=4 {
        assert_eq!(
            RelayCodec::new(None, version).encode(&a_signature_over_a_chosen_document()),
            "+/+/|JVBERg==|eyJmaWxlbmFtZSI6ICJkb2N1bWVudG8udHh0In0=",
            "ver={version}"
        );
    }
}

#[test]
fn with_a_key_the_chosen_document_is_ciphered_on_its_own_and_recoverable() {
    let key = a_key();

    let wire = RelayCodec::new(Some(key.clone()), 3).encode(&a_signature_over_a_chosen_document());
    let fields: Vec<&str> = wire.split(RESULT_SEPARATOR).collect();

    assert_eq!(fields.len(), 3, "{wire}");
    assert_eq!(
        decrypt(fields[2], Some(&key)).expect("descifra"),
        br#"{"filename": "documento.txt"}"#
    );
}

#[test]
fn without_a_key_a_batch_travels_in_plain_base64_per_field() {
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };

    assert_eq!(RelayCodec::new(None, 1).encode(&outcome), "PHhtbC8+|+/+/");
}

#[test]
fn with_a_key_each_batch_field_is_ciphered_on_its_own_and_recoverable() {
    let key = a_key();
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };

    let wire = RelayCodec::new(Some(key.clone()), 1).encode(&outcome);
    let (result, signer) = wire.split_once(RESULT_SEPARATOR).expect("dos campos");

    assert_eq!(decrypt(result, Some(&key)).expect("descifra"), b"<xml/>");
    assert_eq!(
        decrypt(signer, Some(&key)).expect("descifra"),
        vec![0xfb, 0xff, 0xbf]
    );
}

#[test]
fn a_save_goes_out_as_a_plain_ok_never_ciphered() {
    let key = a_key();

    assert_eq!(RelayCodec::new(None, 1).encode(&SiteOutcome::Saved), "OK");
    assert_eq!(
        RelayCodec::new(Some(key), 1).encode(&SiteOutcome::Saved),
        "OK"
    );
}

#[test]
fn a_load_ciphers_its_content_but_never_its_name() {
    let key = a_key();
    let outcome = SiteOutcome::Loaded(vec![("firma.pdf".to_owned(), b"%PDF".to_vec())]);

    let wire = RelayCodec::new(Some(key.clone()), 1).encode(&outcome);
    let (name, content) = wire.split_once(':').expect("nombre y contenido");

    assert_eq!(name, "firma.pdf");
    assert_eq!(decrypt(content, Some(&key)).expect("descifra"), b"%PDF");
}

#[test]
fn errors_travel_in_plain_text_even_with_a_key() {
    let key = a_key();
    let codec = RelayCodec::new(Some(key), 1);

    assert_eq!(codec.encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = codec.encode(&SiteOutcome::Refused(SiteRefusal::ScratchUnwritable(
        "detalle que no sale".to_owned(),
    )));
    assert!(refused.starts_with("SAF_"), "{refused}");
}
