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
    let codec = RelayCodec::new(None);

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
        signed: b"%PDF".to_vec(),
    };

    assert_eq!(RelayCodec::new(None).encode(&outcome), "+/+/|JVBERg==");
}

#[test]
fn with_a_key_each_field_is_ciphered_on_its_own_and_recoverable() {
    let key = a_key();
    let outcome = SiteOutcome::Signature {
        signer_der: vec![0xfb, 0xff, 0xbf],
        signed: b"%PDF".to_vec(),
    };

    let wire = RelayCodec::new(Some(key.clone())).encode(&outcome);
    let (signer, signed) = wire.split_once(RESULT_SEPARATOR).expect("dos campos");

    assert_eq!(
        decrypt(signer, Some(&key)).expect("descifra"),
        vec![0xfb, 0xff, 0xbf]
    );
    assert_eq!(decrypt(signed, Some(&key)).expect("descifra"), b"%PDF");
}

#[test]
fn without_a_key_a_batch_travels_in_plain_base64_per_field() {
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };

    assert_eq!(RelayCodec::new(None).encode(&outcome), "PHhtbC8+|+/+/");
}

#[test]
fn with_a_key_each_batch_field_is_ciphered_on_its_own_and_recoverable() {
    let key = a_key();
    let outcome = SiteOutcome::Batch {
        result: b"<xml/>".to_vec(),
        signer_der: Some(vec![0xfb, 0xff, 0xbf]),
    };

    let wire = RelayCodec::new(Some(key.clone())).encode(&outcome);
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

    assert_eq!(RelayCodec::new(None).encode(&SiteOutcome::Saved), "OK");
    assert_eq!(RelayCodec::new(Some(key)).encode(&SiteOutcome::Saved), "OK");
}

#[test]
fn a_load_ciphers_its_content_but_never_its_name() {
    let key = a_key();
    let outcome = SiteOutcome::Loaded(vec![("firma.pdf".to_owned(), b"%PDF".to_vec())]);

    let wire = RelayCodec::new(Some(key.clone())).encode(&outcome);
    let (name, content) = wire.split_once(':').expect("nombre y contenido");

    assert_eq!(name, "firma.pdf");
    assert_eq!(decrypt(content, Some(&key)).expect("descifra"), b"%PDF");
}

#[test]
fn errors_travel_in_plain_text_even_with_a_key() {
    let key = a_key();
    let codec = RelayCodec::new(Some(key));

    assert_eq!(codec.encode(&SiteOutcome::Cancelled), "CANCEL");
    let refused = codec.encode(&SiteOutcome::Refused(SiteRefusal::ScratchUnwritable(
        "detalle que no sale".to_owned(),
    )));
    assert!(refused.starts_with("SAF_"), "{refused}");
}
