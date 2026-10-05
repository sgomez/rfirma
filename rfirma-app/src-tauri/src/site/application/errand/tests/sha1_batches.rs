//! Pruebas de los lotes de sede que piden SHA-1, con la preferencia «Permitir SHA-1» apagada y activada.

use std::path::Path;
use std::sync::Arc;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::identity::domain::algorithm::SignatureAlgorithm;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::tests::a_memory;
use crate::signing::domain::bridge::Format;
use crate::site::application::errand::*;
use crate::site::application::session::SiteRefusal;
use crate::site::application::tests::InMemoryBatchServices;
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage, SafCode};
use base64::Engine as _;

const A_PRESIGN_WITH_TWO_SIGNS: &[u8] = b"{\"td\":{\"format\":\"XAdES\",\"signinfo\":[{\"id\":\"001\",\"params\":{\"PRE\":\"QUJD\"}},{\"id\":\"002\",\"params\":{\"PRE\":\"REVG\"}}]}}";

const A_PRESIGN_IN_XML: &str = "<xml>\n <firmas format=\"XAdES\">\n  <firma Id=\"001\">\n   <param n=\"PRE\">QUJD</param>\n  </firma>\n </firmas>\n</xml>";

fn remembering_the_preference(
    home: &Path,
    allowed: bool,
) -> crate::signing::adapters::memory::Memory {
    let memory = a_memory(home);
    memory
        .remember_configuration(&Configuration {
            allow_sha1: allowed,
            ..Configuration::default()
        })
        .expect("la memoria de pruebas escribe");
    memory
}

fn a_remote_batch(lote: &str, json: bool) -> AfirmaUrl {
    let json_flag = if json { "&jsonbatch=true" } else { "" };
    let text = format!(
        "afirma://batch?op=batch&idsession={CREDENTIAL}{json_flag}&\
         batchpresignerurl=https%3A%2F%2Fpresigner.example%2Fpre&\
         batchpostsignerurl=https%3A%2F%2Fpostsigner.example%2Fpost&dat={}",
        base64::engine::general_purpose::URL_SAFE.encode(lote)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

/// Un lote remoto con SHA-1 y un elemento XAdES, en JSON y en el XML heredado, con su prefirma.
fn remote_batches_with_sha1() -> Vec<(AfirmaUrl, Vec<u8>, &'static str)> {
    vec![
        (
            a_remote_batch(
                "{\"algorithm\":\"SHA1withRSA\",\"stoponerror\":false,\"singlesigns\":[\
                 {\"id\":\"001\",\"datareference\":\"AAAA\",\"format\":\"XAdES\"},\
                 {\"id\":\"002\",\"datareference\":\"BBBB\"}]}",
                true,
            ),
            A_PRESIGN_WITH_TWO_SIGNS.to_vec(),
            "lote JSON",
        ),
        (
            a_remote_batch(
                "<signbatch algorithm=\"SHA1\" stoponerror=\"false\">\
                 <singlesign id=\"001\" format=\"XAdES\"/></signbatch>",
                false,
            ),
            A_PRESIGN_IN_XML.as_bytes().to_vec(),
            "lote XML",
        ),
    ]
}

/// Lo que pasa con un lote remoto: el consentimiento, lo que firmó el token y cómo acabó.
struct RemoteBatch {
    consent: Box<BatchConsent>,
    signed_with: Vec<String>,
    finished: Result<(), ConsentError>,
}

fn the_remote_batch_with_the_preference(
    url: &AfirmaUrl,
    presign: Vec<u8>,
    allowed: bool,
) -> RemoteBatch {
    the_remote_batch(url, presign, allowed, false)
}

fn the_remote_batch(
    url: &AfirmaUrl,
    presign: Vec<u8>,
    allowed: bool,
    allowed_once: bool,
) -> RemoteBatch {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = remembering_the_preference(home.path(), allowed);
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, _wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let services = Arc::new(InMemoryBatchServices::answering(
        presign,
        b"RESULTADO".to_vec(),
    ));
    let neighbours = a_signer_for_the_batch(home.path(), &listed, &memory, &ours);
    let desk = a_desk_for_the_batch(&engine, &policies, &neighbours, home.path(), services);

    let step = attend_operation(&desk, url, decoded(url), &live);
    let ErrandStep::AskingToSignTheBatch(asked) = remembered(&live, step) else {
        panic!("un lote pide consentimiento");
    };
    if allowed_once {
        live.allow_sha1_once();
    }
    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");
    let finished = finish_the_batch(&desk, &the_typed_secret(), &live);

    RemoteBatch {
        consent: asked,
        signed_with: neighbours
            .neighbours
            .token
            .signed()
            .into_iter()
            .map(|(algorithm, _)| algorithm)
            .collect(),
        finished,
    }
}

#[test]
fn without_the_preference_a_remote_batch_with_sha1_is_refused_as_sha1_and_the_token_signs_nothing()
{
    for (url, presign, what) in remote_batches_with_sha1() {
        let batch = the_remote_batch_with_the_preference(&url, presign, false);

        let Err(ConsentError::Refused(SiteRefusal::BatchSigningFailed(failed))) = batch.finished
        else {
            panic!("{what}: la firma del lote falla");
        };
        assert_eq!(failed.situation, "sha1", "{what}");
        assert_eq!(failed.code, SafCode::BatchSignature, "{what}");
        assert!(batch.signed_with.is_empty(), "{what}: el token no firma");
        assert!(!batch.consent.sha1_allowed, "{what}: sin la marca");
    }
}

#[test]
fn with_the_preference_a_remote_batch_signs_every_pk1_with_sha1_even_with_an_xades_item() {
    for (url, presign, what) in remote_batches_with_sha1() {
        let batch = the_remote_batch_with_the_preference(&url, presign, true);

        batch.finished.expect("el lote sale entero");
        assert!(!batch.signed_with.is_empty(), "{what}: el token firma");
        assert!(
            batch
                .signed_with
                .iter()
                .all(|algorithm| algorithm.starts_with("SHA1")),
            "{what}: cada PK1 se firma con SHA-1: {:?}",
            batch.signed_with
        );
        assert!(batch.consent.sha1_allowed, "{what}: con la marca");
    }
}

#[test]
fn a_remote_batch_with_sha2_carries_no_sha1_mark() {
    let url = a_remote_batch(
        "{\"algorithm\":\"SHA256\",\"singlesigns\":[{\"id\":\"001\",\"datareference\":\"AAAA\"}]}",
        true,
    );

    let batch = the_remote_batch_with_the_preference(&url, A_PRESIGN_WITH_TWO_SIGNS.to_vec(), true);

    assert!(!batch.consent.sha1_allowed);
}

/// Un lote local con SHA-1 de tres elementos: un PDF, un binario y un XML que se firmaría en XAdES.
fn a_local_batch_with_sha1(stop_on_error: bool) -> AfirmaUrl {
    let lote = format!(
        "{{\"algorithm\":\"SHA1withRSA\",\"format\":\"auto\",\"stoponerror\":{stop_on_error},\"singlesigns\":[\
         {{\"id\":\"001\",\"datareference\":\"{}\"}},\
         {{\"id\":\"002\",\"datareference\":\"{}\"}},\
         {{\"id\":\"003\",\"datareference\":\"{}\"}}]}}",
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

/// Lo que pasa con un lote local: consentimiento, firmas del token, cruces al puente y resultado.
struct LocalBatchRun {
    consent: Box<LocalBatchConsent>,
    signed_with: Vec<SignatureAlgorithm>,
    presigned: Vec<Format>,
    finished: Result<(), ConsentError>,
    result: Option<String>,
}

fn the_local_batch_with_the_preference(url: &AfirmaUrl, allowed: bool) -> LocalBatchRun {
    the_local_batch(url, allowed, false)
}

fn the_local_batch(url: &AfirmaUrl, allowed: bool, allowed_once: bool) -> LocalBatchRun {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = remembering_the_preference(home.path(), allowed);
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let live = a_live();
    if allowed_once {
        live.allow_sha1_once();
    }
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let neighbours = neighbours_for_the_local_batch(home.path(), &listed, &opened, &memory, &ours);
    let desk = a_desk(&engine, &policies, &neighbours, &scratch);

    let step = attend_operation(&desk, url, decoded(url), &live);
    let ErrandStep::AskingToSignTheLocalBatch(asked) = remembered(&live, step) else {
        panic!("un lote local pide consentimiento");
    };
    consent(&desk, &asked.certificates[0].id, &live).expect("el certificado sirve");
    let finished = finish_the_local_batch(&desk, &the_typed_secret(), &live);
    let result = finished.is_ok().then(|| the_batch_result(&mut wire));

    LocalBatchRun {
        consent: asked,
        signed_with: neighbours.signer.signed_with(),
        presigned: neighbours.bridge.formats_of_the_presigns(),
        finished,
        result,
    }
}

#[test]
fn without_the_preference_a_local_batch_with_sha1_asks_to_allow_it_and_never_consents_alone() {
    let batch = the_local_batch_with_the_preference(&a_local_batch_with_sha1(false), false);

    assert!(batch.consent.sha1_to_allow, "pide permitirlo");
    assert!(!batch.consent.without_asking, "no consiente solo");
    assert!(!batch.consent.sha1_allowed, "sin la marca");
}

#[test]
fn allowed_once_a_local_batch_signs_every_item_with_sha1_without_the_preference() {
    let batch = the_local_batch(&a_local_batch_with_sha1(false), false, true);

    batch.finished.expect("el lote local contesta");
    assert_eq!(batch.presigned.len(), 3);
    assert_eq!(
        batch.signed_with,
        vec![SignatureAlgorithm::Sha1Ecdsa; 3],
        "cada elemento se firma con SHA-1"
    );
    assert!(!batch.consent.sha1_to_allow);
}

#[test]
fn with_the_preference_a_local_batch_with_sha1_does_not_ask_to_allow_it() {
    let batch = the_local_batch_with_the_preference(&a_local_batch_with_sha1(false), true);

    assert!(!batch.consent.sha1_to_allow);
}

#[test]
fn with_the_preference_a_local_batch_signs_every_item_with_sha1_xades_included() {
    let batch = the_local_batch_with_the_preference(&a_local_batch_with_sha1(false), true);

    batch.finished.expect("el lote local contesta");
    let result = batch.result.expect("la sede recibe el resultado");
    for id in ["001", "002", "003"] {
        assert!(
            result.contains(&format!("\"id\":\"{id}\",\"result\":\"DONE_AND_SAVED\"")),
            "{result}"
        );
    }
    assert_eq!(batch.presigned.len(), 3);
    assert!(matches!(batch.presigned[2], Format::Xades(_)));
    assert_eq!(
        batch.signed_with,
        vec![SignatureAlgorithm::Sha1Ecdsa; 3],
        "el certificado de pruebas lleva clave EC y firma con SHA-1, nunca con SHA-256"
    );
    assert!(batch.consent.sha1_allowed, "con la marca");
}

#[test]
fn a_local_batch_with_sha2_carries_no_sha1_mark() {
    let batch = the_local_batch_with_the_preference(&a_local_batch(""), true);

    assert!(!batch.consent.sha1_allowed);
}

#[test]
fn without_the_preference_a_remote_batch_with_sha1_asks_to_allow_it_and_never_consents_alone() {
    for (url, presign, what) in remote_batches_with_sha1() {
        let batch = the_remote_batch_with_the_preference(&url, presign, false);

        assert!(batch.consent.sha1_to_allow, "{what}: pide permitirlo");
        assert!(!batch.consent.without_asking, "{what}: no consiente solo");
    }
}

#[test]
fn allowed_once_a_remote_batch_signs_every_pk1_with_sha1_without_the_preference() {
    for (url, presign, what) in remote_batches_with_sha1() {
        let batch = the_remote_batch(&url, presign, false, true);

        batch.finished.expect("el lote sale entero");
        assert!(!batch.signed_with.is_empty(), "{what}: el token firma");
        assert!(
            batch
                .signed_with
                .iter()
                .all(|algorithm| algorithm.starts_with("SHA1")),
            "{what}: cada PK1 se firma con SHA-1: {:?}",
            batch.signed_with
        );
    }
}

#[test]
fn with_the_preference_a_remote_batch_with_sha1_does_not_ask_to_allow_it() {
    for (url, presign, what) in remote_batches_with_sha1() {
        let batch = the_remote_batch_with_the_preference(&url, presign, true);

        assert!(!batch.consent.sha1_to_allow, "{what}");
    }
}

#[test]
fn allowing_sha1_once_lasts_only_until_the_operation_ends() {
    let live = a_live();

    live.allow_sha1_once();
    live.end();

    assert!(!live.sha1_allowed_once());
}
