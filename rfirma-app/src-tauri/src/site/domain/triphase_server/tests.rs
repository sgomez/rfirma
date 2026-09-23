use super::*;

use crate::site::domain::batch::TriSign;
use crate::site::domain::protocol::CounterTarget;

fn the_site_declaring(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn a_call(round: SignatureRound) -> ServerCall<'static> {
    ServerCall {
        round,
        algorithm: "SHA256withRSA",
        certificate: b"el certificado",
        document: b"los datos",
        params: None,
    }
}

fn the_value_of<'a>(form: &'a [(&'static str, String)], name: &str) -> Option<&'a str> {
    form.iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.as_str())
}

fn url_safe(bytes: &[u8]) -> String {
    URL_SAFE.encode(bytes)
}

#[test]
fn the_server_url_is_read_from_the_properties() {
    let declared = the_site_declaring(&[("serverUrl", "https://sede.example/tri")]);

    assert_eq!(
        server_url_of(&declared).expect("la sede lo declara"),
        "https://sede.example/tri"
    );
}

#[test]
fn a_signature_without_server_url_is_missing_a_parameter() {
    let error = server_url_of(&BTreeMap::new()).expect_err("no hay servidor al que llamar");

    assert_eq!(error.situation(), Situation::ServerUrlMissing);
}

#[test]
fn a_server_url_that_is_not_http_is_as_good_as_missing() {
    for declared in ["", "sede.example/tri", "file:///etc/passwd", "https://"] {
        let error = server_url_of(&the_site_declaring(&[("serverUrl", declared)]))
            .expect_err("no es una URL a la que se pueda llamar");

        assert_eq!(error.situation(), Situation::ServerUrlMissing, "{declared}");
    }
}

#[test]
fn the_server_does_not_get_back_the_server_url_nor_the_document_id() {
    let declared = the_site_declaring(&[
        ("serverUrl", "https://sede.example/tri"),
        ("documentId", "abc"),
        ("mode", "implicit"),
    ]);

    let params = params_for_the_server(&declared, SignatureRound::First);

    assert_eq!(params, the_site_declaring(&[("mode", "implicit")]));
}

#[test]
fn a_countersignature_tells_the_server_its_target() {
    let params = params_for_the_server(
        &BTreeMap::new(),
        SignatureRound::Counter {
            target: CounterTarget::Tree,
        },
    );

    assert_eq!(params, the_site_declaring(&[("target", "tree")]));
}

#[test]
fn the_presign_asks_for_pre_with_the_operation_the_cades_format_and_the_data() {
    let form = presign_form(&a_call(SignatureRound::Again));

    assert_eq!(the_value_of(&form, "op"), Some("pre"));
    assert_eq!(the_value_of(&form, "cop"), Some("cosign"));
    assert_eq!(the_value_of(&form, "format"), Some("CAdES"));
    assert_eq!(the_value_of(&form, "algo"), Some("SHA256withRSA"));
    assert_eq!(
        the_value_of(&form, "cert"),
        Some(url_safe(b"el certificado").as_str())
    );
    assert_eq!(
        the_value_of(&form, "doc"),
        Some(url_safe(b"los datos").as_str())
    );
    assert_eq!(the_value_of(&form, "params"), None);
}

#[test]
fn the_params_travel_as_base64_of_the_properties() {
    let call = ServerCall {
        params: Some("mode=implicit\n"),
        ..a_call(SignatureRound::First)
    };

    let form = presign_form(&call);

    assert_eq!(
        the_value_of(&form, "params"),
        Some(url_safe(b"mode=implicit\n").as_str())
    );
}

#[test]
fn the_postsign_carries_the_signed_session_and_the_data() {
    let signed = TriphaseData::new(
        Some("CAdES".to_owned()),
        vec![TriSign::new(
            Some("1".to_owned()),
            None,
            vec![("PK1".to_owned(), "cGsx".to_owned())],
        )],
    );

    let form = postsign_form(
        &a_call(SignatureRound::Counter {
            target: CounterTarget::Leafs,
        }),
        &signed,
    );

    assert_eq!(the_value_of(&form, "op"), Some("post"));
    assert_eq!(the_value_of(&form, "cop"), Some("countersign"));
    assert_eq!(
        the_value_of(&form, "session"),
        Some(url_safe(signed.to_xml().as_bytes()).as_str())
    );
    assert_eq!(
        the_value_of(&form, "doc"),
        Some(url_safe(b"los datos").as_str())
    );
}

#[test]
fn a_presign_answer_is_the_session_in_base64() {
    let xml = "<xml>\n <firmas format=\"CAdES\">\n  <firma Id=\"1\">\n   <param n=\"PRE\">cHJl</param>\n  </firma>\n </firmas>\n</xml>";

    let session = presigned(url_safe(xml.as_bytes()).as_bytes()).expect("es una prefirma");

    assert_eq!(session.signs()[0].param("PRE"), Some("cHJl"));
}

#[test]
fn an_error_of_the_server_that_carries_its_exception_is_a_server_exception() {
    let error = presigned(b"ERR-14:prefirma:java.io.IOException: la sede no entrega el documento")
        .expect_err("el servidor fallo");

    assert_eq!(error.situation(), Situation::ServerException);
    assert!(error.detail().contains("IOException"));
}

#[test]
fn an_error_of_the_server_without_an_exception_is_an_unexpected_answer() {
    let error = presigned(b"ERR-14:sin excepcion").expect_err("el servidor fallo");

    assert_eq!(error.situation(), Situation::UnexpectedAnswer);
}

#[test]
fn a_presign_answer_that_is_not_a_session_is_an_unexpected_answer() {
    let error = presigned(b"esto no es base64 de nada").expect_err("no es una prefirma");

    assert_eq!(error.situation(), Situation::UnexpectedAnswer);
}

#[test]
fn the_postsign_answer_hands_back_the_signature_after_newid() {
    let answer = format!("OK NEWID={}\n", url_safe(b"la firma"));

    assert_eq!(
        postsigned(answer.as_bytes()).expect("la postfirma acabo bien"),
        b"la firma"
    );
}

#[test]
fn a_postsign_answer_that_is_not_ok_is_an_unexpected_answer() {
    let error = postsigned(b"ERR-15:postfirma").expect_err("no acabo bien");

    assert_eq!(error.situation(), Situation::UnexpectedAnswer);
}

#[test]
fn a_postsign_error_that_needs_configuration_and_carries_an_exception_is_a_server_exception() {
    let error = postsigned(b"ERR-21:CODE:es.gob.afirma.core.RuntimeConfigNeededException: pide")
        .expect_err("no acabo bien");

    assert_eq!(error.situation(), Situation::ServerException);
}
