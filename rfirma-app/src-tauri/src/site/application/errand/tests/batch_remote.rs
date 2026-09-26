//! Pruebas del lote remoto de sede.

use std::sync::Arc;

use super::support::*;
use super::support_requests::*;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::ports::CertificateMemory;
use crate::signing::application::tests::a_memory;
use crate::site::adapters::frontier;
use crate::site::application::errand::*;
use crate::site::application::tests::{InMemoryBatchServices, ReceivedBatchCall};
use crate::site::domain::batch::BatchFormat;
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage, SafCode, WireAnswer};
use base64::Engine as _;

const A_JSON_LOTE: &str = "{\"algorithm\":\"SHA256\",\"stoponerror\":false,\"singlesigns\":[{\"id\":\"001\",\"datareference\":\"AAAA\"},{\"id\":\"002\",\"datareference\":\"BBBB\"}]}";

const A_PRESIGN_WITH_TWO_SIGNS: &[u8] = b"{\"td\":{\"format\":\"PAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"PRE\":\"QUJD\"}},{\"id\":\"002\",\"params\":{\"PRE\":\"REVG\"}}]}}";

fn a_batch(extra: &str) -> AfirmaUrl {
    let text = format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonbatch=true&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}{extra}",
        base64::engine::general_purpose::URL_SAFE.encode(A_JSON_LOTE)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// El asa del único certificado que la sede acepta en ese paso.
fn the_only_row_of(step: ErrandStep) -> String {
    let ErrandStep::AskingToSignTheBatch(asked) = step else {
        panic!("un lote pide consentimiento: {step:?}");
    };
    asked.certificates[0].id.clone()
}

#[test]
fn a_batch_goes_from_the_operation_to_the_wire_asking_the_secret_only_once() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let services = Arc::new(InMemoryBatchServices::answering(
        A_PRESIGN_WITH_TWO_SIGNS.to_vec(),
        b"RESULTADO".to_vec(),
    ));
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::clone(&services),
    );

    let url = a_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheBatch(asked) = remembered(&live, step) else {
        panic!("un lote pide consentimiento");
    };
    assert_eq!(asked.signs, 2, "el momento dice cuantas firmas lleva");
    assert_eq!(asked.certificates.len(), 1);
    assert_eq!(asked.already_chosen, None, "sin 'sticky' no hay elegido");
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "el consentimiento del lote no escribe nada en el cable"
    );

    let consented = consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");
    assert!(matches!(consented, Consented::SigningWith(_)));

    finish_the_batch(&desk, &the_typed_secret(), &live).expect("el lote sale entero");

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(base64::engine::general_purpose::STANDARD.encode(b"RESULTADO")),
        "la sede recibe el resultado del postsigner tal cual"
    );
    assert_eq!(
        desk.neighbours.neighbours.token.secrets_asked(),
        1,
        "el secreto se pide una sola vez para las dos firmas"
    );
    assert_eq!(desk.neighbours.neighbours.token.signed().len(), 2);
    assert_eq!(services.received().len(), 2);
    assert!(live.current().is_none());
}

#[test]
fn a_batch_with_needcert_answers_the_result_and_the_signer() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let services = Arc::new(InMemoryBatchServices::answering(
        A_PRESIGN_WITH_TWO_SIGNS.to_vec(),
        b"RESULTADO".to_vec(),
    ));
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        services,
    );

    let url = a_batch("&needcert=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");
    finish_the_batch(&desk, &the_typed_secret(), &live).expect("el lote sale entero");

    let encode = base64::engine::general_purpose::STANDARD;
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(format!(
            "{}|{}",
            encode.encode(b"RESULTADO"),
            encode.encode(ours[0].der())
        ))
    );
}

#[test]
fn a_sticky_batch_does_not_take_the_desk_remembered_certificate_as_already_chosen() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    memory
        .remember_the_certificate(ours[0].reference())
        .expect("la memoria de pruebas escribe");
    let live = a_live();
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

    let url = a_batch("&sticky=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheBatch(asked) = step else {
        panic!("un lote pega el certificado, no lo contesta");
    };

    assert_eq!(
        asked.already_chosen, None,
        "sin nada fijado en la sesion no hay elegido"
    );
}

#[test]
fn a_sticky_batch_leaves_the_certificate_stuck_in_its_session_already_chosen_in_the_step() {
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

    let url = a_batch("&sticky=true");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let ErrandStep::AskingToSignTheBatch(asked) = step else {
        panic!("un lote pega el certificado, no lo contesta");
    };

    assert_eq!(
        asked.already_chosen.as_deref(),
        Some(asked.certificates[0].id.as_str()),
        "'sticky' resuelve el certificado sin preguntar"
    );
    assert!(asked.certificates[0].remembered);
}

#[test]
fn a_batch_whose_presigner_is_unreachable_is_answered_with_the_code_of_the_batch_service() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::new(InMemoryBatchServices::unreachable()),
    );

    let url = a_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");

    let refused =
        finish_the_batch(&desk, &the_typed_secret(), &live).expect_err("sin servlet no hay lote");

    assert!(matches!(refused, ConsentError::Refused(_)));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::ContactBatchService).on_the_wire())
    );
}

#[test]
fn a_batch_whose_postsigner_answers_nothing_is_answered_with_the_code_of_a_failed_batch() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::new(InMemoryBatchServices::only_presigning(
            A_PRESIGN_WITH_TWO_SIGNS.to_vec(),
        )),
    );

    let url = a_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");

    finish_the_batch(&desk, &the_typed_secret(), &live)
        .expect_err("una postfirma invalida no sale");

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::BatchSignature).on_the_wire())
    );
}

#[test]
fn a_json_batch_asked_with_jsonbatch_capitalised_reaches_the_presigner_as_the_legacy_xml() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let services = Arc::new(InMemoryBatchServices::default());
    let desk = a_desk_for_the_batch(
        &engine,
        &policies,
        home.path(),
        &listed,
        &memory,
        &ours,
        Arc::clone(&services),
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}&jsonBatch=true&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        base64::engine::general_purpose::URL_SAFE.encode(A_JSON_LOTE)
    )) else {
        panic!("una URL del protocolo es una operacion");
    };

    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");
    finish_the_batch(&desk, &the_typed_secret(), &live)
        .expect_err("el prefirmador rechaza el lote");

    assert!(matches!(
        services.received().first(),
        Some(ReceivedBatchCall::Presign {
            format: BatchFormat::Xml,
            ..
        })
    ));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::BatchSignature).on_the_wire())
    );
}

#[test]
fn a_batch_that_is_declined_ends_in_a_cancel() {
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

    let url = a_batch("");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    remembered(&live, step);

    decline(&live);

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(frontier::cancelled().on_the_wire())
    );
}
