//! Pruebas del lote local de sede.

use std::sync::Arc;

use crate::site::application::errand::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::signing::domain::bridge::{
    Format, XadesVariant,
};
use crate::site::adapters::frontier;
use crate::site::application::tests::InMemoryBatchServices;
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, SignatureRound,
};
use base64::Engine as _;
use super::support::*;
use super::support_requests::*;

/// Un lote local de tres elementos donde el segundo pide un formato que el puente no atiende
/// (`cades-asic-s`), para ejercitar el fallo de un elemento sin depender del puente doblado.
fn a_local_batch_with_a_failing_second_item(stop_on_error: bool) -> AfirmaUrl {
    let lote = format!(
        "{{\"algorithm\":\"SHA256\",\"format\":\"auto\",\"stoponerror\":{},\"singlesigns\":[\
         {{\"id\":\"001\",\"datareference\":\"{}\"}},\
         {{\"id\":\"002\",\"datareference\":\"{}\",\"format\":\"pades\"}},\
         {{\"id\":\"003\",\"datareference\":\"{}\"}}]}}",
        stop_on_error,
        in_the_batch(A_LOCAL_PDF),
        in_the_batch(A_LOCAL_BINARY),
        in_the_batch(A_LOCAL_XML),
    );
    let text = format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonbatch=true&\
         localBatchProcess=true&dat={}",
        base64::engine::general_purpose::URL_SAFE.encode(&lote)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn a_local_batch_reaches_the_consent_with_a_summary_of_every_item() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_local_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };

    assert_eq!(
        asked.items.len(),
        3,
        "el momento dice que el lote lleva tres"
    );
    assert_eq!(asked.items[0].id, "001");
    assert_eq!(
        asked.items[0].format,
        Format::Pades,
        "la cabecera es un PDF"
    );
    assert_eq!(asked.items[0].round, SignatureRound::First);
    assert_eq!(asked.items[1].id, "002");
    assert_eq!(asked.items[1].format, Format::Cades, "ni PDF ni XML");
    assert_eq!(asked.items[1].round, SignatureRound::Again);
    assert_eq!(asked.items[2].id, "003");
    assert!(matches!(asked.items[2].format, Format::Xades(_)));
    assert_eq!(asked.already_chosen, None, "sin 'sticky' no hay elegido");
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el consentimiento del lote local no escribe nada en el cable"
    );

    let consented = consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");
    assert!(matches!(consented, Consented::SigningWith(_)));

    finish_the_local_batch(&desk, "1234", &live).expect("el lote local contesta");

    assert_eq!(
        desk.neighbours.token.secrets_asked(),
        1,
        "el secreto se pide una sola vez para las tres firmas"
    );
    assert_eq!(
        desk.neighbours.bridge.formats_of_the_presigns(),
        vec![
            Format::Pades,
            Format::Cades,
            Format::Xades(XadesVariant::Enveloping)
        ],
        "cada elemento cruza al puente con su propio formato"
    );
    let result = the_batch_result(&mut wire);
    assert_eq!(result.matches("\"result\":\"DONE_AND_SAVED\"").count(), 3);
    assert!(result.contains("\"id\":\"001\""));
    assert!(result.contains("\"id\":\"002\""));
    assert!(result.contains("\"id\":\"003\""));
    assert!(live.current().is_none());
    assert!(
        std::fs::read_dir(&scratch)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true),
        "cada elemento borra su documento de paso al firmarlo"
    );
}

#[test]
fn a_local_batch_that_stops_on_error_skips_what_came_before_and_after() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_local_batch_with_a_failing_second_item(true);
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };
    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");

    finish_the_local_batch(&desk, "1234", &live).expect("el lote local contesta");

    let result = the_batch_result(&mut wire);
    assert!(
        result.contains("\"id\":\"001\",\"result\":\"SKIPPED\""),
        "{result}"
    );
    assert!(
        result.contains("\"id\":\"002\",\"result\":\"ERROR_PRE\""),
        "{result}"
    );
    assert!(
        result.contains("\"id\":\"003\",\"result\":\"SKIPPED\""),
        "{result}"
    );
}

#[test]
fn a_local_batch_without_stoponerror_signs_around_the_failure() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_local_batch_with_a_failing_second_item(false);
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };
    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");

    finish_the_local_batch(&desk, "1234", &live).expect("el lote local contesta");

    let result = the_batch_result(&mut wire);
    assert!(
        result.contains("\"id\":\"001\",\"result\":\"DONE_AND_SAVED\""),
        "{result}"
    );
    assert!(
        result.contains("\"id\":\"002\",\"result\":\"ERROR_PRE\""),
        "{result}"
    );
    assert!(
        result.contains("\"id\":\"003\",\"result\":\"DONE_AND_SAVED\""),
        "{result}"
    );
}

#[test]
fn a_local_batch_with_needcert_answers_the_signer() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk_for_the_local_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
        &ours,
    );

    let url = a_local_batch("&needcert=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };
    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");

    finish_the_local_batch(&desk, "1234", &live).expect("el lote local contesta");

    let answered = what_the_site_received(&mut wire).expect("la sede recibe el resultado del lote");
    let (result, signer) = answered
        .split_once('|')
        .expect("needcert añade el DER tras el resultado, separado por '|'");
    assert_eq!(
        signer,
        base64::engine::general_purpose::STANDARD.encode(ours[0].der())
    );
    assert!(!result.is_empty());
}

#[test]
fn a_sticky_local_batch_leaves_the_certificate_stuck_in_its_session_already_chosen_in_the_step() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    live.stick(ours[0].reference());
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::new(InMemoryBatchServices::default()),
    );

    let url = a_local_batch("&sticky=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = step else {
        panic!("un lote local pega el certificado, no lo contesta");
    };

    assert_eq!(
        asked.already_chosen.as_deref(),
        Some(asked.certificates[0].id.as_str()),
        "'sticky' resuelve el certificado sin preguntar"
    );
    assert!(asked.certificates[0].remembered);
}

#[test]
fn a_local_batch_that_is_declined_ends_in_a_cancel() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::new(InMemoryBatchServices::default()),
    );

    let url = a_local_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    remembered(&live, step);

    decline(&live);

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(frontier::cancelled().on_the_wire())
    );
}

