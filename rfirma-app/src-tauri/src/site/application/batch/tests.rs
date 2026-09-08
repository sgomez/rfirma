//! Pruebas del lote remoto ya consentido, con los dos servlets y el token doblados.

use super::*;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::identity::application::tests::a_usable_certificate;
use crate::site::application::tests::{
    InMemoryBatchServices, InMemoryTokenSigning, ReceivedBatchCall,
};
use crate::site::domain::batch_error::Situation;
use crate::site::domain::protocol::{read_operation, AfirmaUrl, SiteOperation};

/// Capturado del `afirma-server-triphase-signer` de pruebas: un `td` con una firma y un error.
const PRESIGN_WITH_ONE_ERROR: &[u8] = b"{\"td\":{\"format\":\"PAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"PRE\":\"QUJD\"}}]},\"results\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}";

const PRESIGN_WITH_TWO_SIGNS: &[u8] = b"{\"td\":{\"format\":\"PAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"PRE\":\"QUJD\"}},{\"id\":\"002\",\"params\":{\"PRE\":\"REVG\"}}]}}";

const PRESIGN_XML: &str = "<xml>\n <firmas format=\"PAdES\">\n  <firma Id=\"001\">\n   <param n=\"PRE\">QUJD</param>\n  </firma>\n </firmas>\n</xml>";

const JSON_LOTE: &str = "{\"algorithm\":\"SHA256\",\"stoponerror\":false,\"singlesigns\":[{\"id\":\"001\",\"datareference\":\"AAAA\"},{\"id\":\"002\",\"datareference\":\"BBBB\",\"format\":\"PAdES\"}]}";

const XML_LOTE: &str = "<signbatch algorithm=\"SHA256\" stoponerror=\"false\"><singlesign id=\"001\"/><singlesign id=\"002\"/></signbatch>";

fn a_batch_request(lote: &str, json: bool) -> crate::site::domain::protocol::BatchRequest {
    let json_flag = match json {
        true => "&jsonbatch=true",
        false => "",
    };
    let text = format!(
        "afirma://batch?op=batch&idsession=8jAkPZfRw2mQxN4TbYuL&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost{json_flag}&dat={}",
        URL_SAFE_NO_PAD.encode(lote)
    );
    let url = AfirmaUrl::parse(&text).expect("es una URL del protocolo");
    let SiteOperation::Batch(request) = read_operation(&url).expect("es un lote que se atiende")
    else {
        panic!("es un lote");
    };
    request
}

fn a_run<'a>(
    services: &'a InMemoryBatchServices,
    token: &'a InMemoryTokenSigning,
    certificate: &'a TokenCertificate,
) -> BatchRun<'a> {
    BatchRun {
        services,
        token,
        certificate,
        secret: "1234",
    }
}

#[test]
fn a_json_batch_reaches_both_servlets_with_the_pk1_of_every_presigned_sign() {
    let services =
        InMemoryBatchServices::answering(PRESIGN_WITH_TWO_SIGNS.to_vec(), b"RESULTADO".to_vec());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let result = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect("el lote sale entero");

    assert_eq!(result, b"RESULTADO");
    let received = services.received();
    let ReceivedBatchCall::Presign {
        url,
        format,
        lote_base64,
        certs,
    } = &received[0]
    else {
        panic!("la primera llamada es la prefirma");
    };
    assert_eq!(url, "https://presigner.example/pre");
    assert_eq!(*format, BatchFormat::Json);
    assert_eq!(lote_base64, &URL_SAFE_NO_PAD.encode(JSON_LOTE));
    assert_eq!(certs, &vec![certificate.der().to_vec()]);

    let ReceivedBatchCall::Postsign { url, tridata, .. } = &received[1] else {
        panic!("la segunda llamada es la postfirma");
    };
    assert_eq!(url, "https://postsigner.example/post");
    assert_eq!(tridata.signs()[0].param("PK1"), Some("UEsxOkFCQw=="));
    assert_eq!(tridata.signs()[1].param("PK1"), Some("UEsxOkRFRg=="));
    assert_eq!(
        token.signed(),
        vec![
            ("SHA256".to_owned(), b"ABC".to_vec()),
            ("SHA256".to_owned(), b"DEF".to_vec())
        ]
    );
}

#[test]
fn an_xml_batch_travels_as_xml_with_the_triphase_data_of_the_legacy_presigner() {
    let services =
        InMemoryBatchServices::answering(PRESIGN_XML.as_bytes().to_vec(), b"<resultado/>".to_vec());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(XML_LOTE, false);

    let result = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect("el lote sale entero");

    assert_eq!(result, b"<resultado/>");
    let received = services.received();
    let ReceivedBatchCall::Postsign {
        format,
        lote_base64,
        tridata,
        ..
    } = &received[1]
    else {
        panic!("la segunda llamada es la postfirma");
    };
    assert_eq!(*format, BatchFormat::Xml);
    assert_eq!(lote_base64, &URL_SAFE_NO_PAD.encode(XML_LOTE));
    assert_eq!(tridata.signs()[0].param("PK1"), Some("UEsxOkFCQw=="));
}

#[test]
fn a_presign_with_errors_sends_the_updated_batch_to_the_postsigner() {
    let services =
        InMemoryBatchServices::answering(PRESIGN_WITH_ONE_ERROR.to_vec(), b"RESULTADO".to_vec());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    signed_batch(&a_run(&services, &token, &certificate), &request).expect("el lote sale entero");

    let received = services.received();
    let ReceivedBatchCall::Postsign { lote_base64, .. } = &received[1] else {
        panic!("la segunda llamada es la postfirma");
    };
    let updated = URL_SAFE_NO_PAD
        .decode(lote_base64)
        .expect("el lote actualizado viaja en base64");
    let updated = String::from_utf8(updated).expect("el lote actualizado es UTF-8");
    assert!(updated.contains("\"result\":\"ERROR_PRE\""));
    assert!(!updated.contains("BBBB"));
    assert!(updated.contains("AAAA"));
}

#[test]
fn a_presign_without_signs_answers_its_errors_without_calling_the_postsigner() {
    let errors =
        b"{\"results\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}";
    let services = InMemoryBatchServices::answering(errors.to_vec(), b"NUNCA".to_vec());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let result = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect("un lote sin prefirmas contesta igual");

    assert_eq!(
        String::from_utf8(result).expect("el resultado es UTF-8"),
        "{\"signs\":[{\"id\":\"002\",\"result\":\"ERROR_PRE\",\"description\":\"fallo\"}]}"
    );
    assert_eq!(services.received().len(), 1);
    assert!(token.signed().is_empty());
}

#[test]
fn a_presign_with_neither_signs_nor_errors_answers_an_empty_result() {
    let services = InMemoryBatchServices::answering(b"{}".to_vec(), b"NUNCA".to_vec());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let result = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect("un lote vacio contesta igual");

    assert_eq!(
        String::from_utf8(result).expect("el resultado es UTF-8"),
        "{\"signs\":[]}"
    );
    assert_eq!(services.received().len(), 1);
}

#[test]
fn a_presigner_that_does_not_answer_is_a_batch_refusal() {
    let services = InMemoryBatchServices::unreachable();
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let refusal = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect_err("sin servlet no hay lote");

    let SiteRefusal::Batch(error) = refusal else {
        panic!("es una situacion del lote");
    };
    assert_eq!(error.situation(), Situation::PresignerUnreachable);
}

#[test]
fn a_presign_response_that_is_not_the_expected_shape_is_an_invalid_presign() {
    let services = InMemoryBatchServices::answering(b"no soy json".to_vec(), Vec::new());
    let token = InMemoryTokenSigning::default();
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let refusal = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect_err("una prefirma ilegible no sigue");

    let SiteRefusal::Batch(error) = refusal else {
        panic!("es una situacion del lote");
    };
    assert_eq!(error.situation(), Situation::InvalidPresignResponse);
}

#[test]
fn a_token_that_refuses_to_sign_stops_the_batch_before_the_postsigner() {
    let services =
        InMemoryBatchServices::answering(PRESIGN_WITH_TWO_SIGNS.to_vec(), b"NUNCA".to_vec());
    let token = InMemoryTokenSigning::refusing(SigningRefusal {
        code: crate::site::domain::protocol::SafCode::CannotAccessKeystore,
        situation: "wrongSecret".to_owned(),
        detail: "el secreto no vale".to_owned(),
        attempts_left: None,
    });
    let certificate = a_usable_certificate("un certificado");
    let request = a_batch_request(JSON_LOTE, true);

    let refusal = signed_batch(&a_run(&services, &token, &certificate), &request)
        .expect_err("sin PK1 no hay postfirma");

    assert!(matches!(refusal, SiteRefusal::BatchSigningFailed(_)));
    assert_eq!(services.received().len(), 1);
}

#[test]
fn the_number_of_signs_is_read_from_the_batch_in_both_formats() {
    assert_eq!(how_many(&a_batch_request(JSON_LOTE, true)), 2);
    assert_eq!(how_many(&a_batch_request(XML_LOTE, false)), 2);
}
