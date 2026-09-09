use base64::engine::general_purpose::STANDARD;

use super::*;
use crate::site::domain::protocol::codes::SafCode;
use crate::site::domain::protocol::XadesEnvelope;

/// **Grada A**: entra el JSON que arma `autoscript.js` y sale el lote leído. No
/// hay socket, ni token, ni puente.
fn a_batch(fields: &str, signs: &str) -> String {
    format!("{{{fields},\"singlesigns\":[{signs}]}}")
}

fn base64(text: &str) -> String {
    STANDARD.encode(text.as_bytes())
}

#[test]
fn a_batch_of_two_reads_the_second_sign_inheriting_what_it_does_not_declare() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"XAdES\",\"suboperation\":\"cosign\",\"stoponerror\":true",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\",\"format\":\"PAdES\",\"suboperation\":\"sign\"}},\
             {{\"id\":\"002\",\"datareference\":\"{}\"}}",
            base64("el primero"),
            base64("el segundo")
        ),
    );

    let batch = parse_local_batch(json.as_bytes()).expect("es un lote que se atiende");

    assert_eq!(batch.algorithm(), "SHA256");
    assert!(batch.stops_on_error());
    let [first, second] = batch.signs() else {
        panic!("el lote declara dos firmas");
    };
    assert_eq!(first.id(), "001");
    assert_eq!(first.document(), b"el primero");
    assert_eq!(first.format(), Some(RequestedFormat::Pades));
    assert_eq!(first.round(), SignatureRound::First);
    assert_eq!(second.id(), "002");
    assert_eq!(second.document(), b"el segundo");
    assert_eq!(
        second.format(),
        Some(RequestedFormat::Xades(XadesEnvelope::Enveloping))
    );
    assert_eq!(second.round(), SignatureRound::Again);
}

#[test]
fn a_batch_without_format_is_refused_with_the_text_of_the_original() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\"",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("el lote no declara formato");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Format));
    assert_eq!(refusal.detail(), "El lote no declaraba el formato de firma");
}

#[test]
fn a_batch_without_algorithm_is_refused_with_the_text_of_the_original() {
    let json = a_batch(
        "\"format\":\"PAdES\"",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("el lote no declara algoritmo");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Algorithm));
    assert_eq!(
        refusal.detail(),
        "El lote no declaraba el algoritmo de firma"
    );
}

#[test]
fn a_sign_without_its_identifier_is_refused() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"PAdES\"",
        &format!("{{\"datareference\":\"{}\"}}", base64("dato")),
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("la firma no trae id");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(
        refusal.detail(),
        "No se ha incluido el identificador de un documento del lote"
    );
}

#[test]
fn a_sign_without_its_data_reference_is_refused() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"PAdES\"",
        "{\"id\":\"001\"}",
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("la firma no trae datareference");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(
        refusal.detail(),
        "No se ha incluido la referencia de un documento del lote"
    );
}

#[test]
fn the_format_auto_of_the_batch_leaves_the_signs_without_format() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"AUTO\"",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let batch = parse_local_batch(json.as_bytes()).expect("'auto' se atiende");

    assert_eq!(batch.signs()[0].format(), None);
}

#[test]
fn a_format_that_autofirma_does_not_sign_in_three_phases_is_refused() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"ODF\"",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("ODF no se firma trifasico");

    assert_eq!(refusal.code(), SafCode::UnsupportedFormat);
}

#[test]
fn a_countersign_suboperation_is_not_attended() {
    let json = a_batch(
        "\"algorithm\":\"SHA256\",\"format\":\"CAdES\",\"suboperation\":\"countersign\"",
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let refusal = parse_local_batch(json.as_bytes()).expect_err("la contrafirma no se atiende");

    assert_eq!(refusal.code(), SafCode::UnsupportedOperation);
}

#[test]
fn a_sign_that_declares_no_extraparams_inherits_those_of_the_batch() {
    let json = a_batch(
        &format!(
            "\"algorithm\":\"SHA256\",\"format\":\"PAdES\",\"extraparams\":\"{}\"",
            base64("signaturePage=1\\nsignatureRotation=90")
        ),
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}},\
             {{\"id\":\"002\",\"datareference\":\"{}\",\"extraparams\":\"{}\"}}",
            base64("dato"),
            base64("otro"),
            base64("signaturePage=2")
        ),
    );

    let batch = parse_local_batch(json.as_bytes()).expect("es un lote que se atiende");

    assert_eq!(
        batch.signs()[0].extra_params(),
        [
            ("signaturePage".to_owned(), "1".to_owned()),
            ("signatureRotation".to_owned(), "90".to_owned())
        ]
    );
    assert_eq!(
        batch.signs()[1].extra_params(),
        [("signaturePage".to_owned(), "2".to_owned())]
    );
}

#[test]
fn a_batch_that_is_not_json_is_refused() {
    let refusal = parse_local_batch(b"<signbatch/>").expect_err("no es JSON");

    assert_eq!(refusal.code(), SafCode::Params);
    assert_eq!(refusal.blame(), Some(Parameter::Data));
}

/// El original borra `profile` antes de firmar, y aquí tampoco cruza cuando viaja dentro de los
/// `extraparams` de un lote (`ProtocolInvocationLauncherSign.java:153`, 1.9.2).
#[test]
fn the_profile_of_the_batch_never_reaches_the_signer() {
    let json = a_batch(
        &format!(
            "\"algorithm\":\"SHA256\",\"format\":\"PAdES\",\"extraparams\":\"{}\"",
            base64("profile=baseline\\nsignaturePage=1")
        ),
        &format!(
            "{{\"id\":\"001\",\"datareference\":\"{}\"}}",
            base64("dato")
        ),
    );

    let batch = parse_local_batch(json.as_bytes()).expect("es un lote que se atiende");

    assert_eq!(
        batch.signs()[0].extra_params(),
        [("signaturePage".to_owned(), "1".to_owned())]
    );
}
