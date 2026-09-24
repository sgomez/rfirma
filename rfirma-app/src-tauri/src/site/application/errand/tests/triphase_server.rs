//! Pruebas de la firma que la sede manda hacer a su servidor trifásico.

use std::sync::Arc;

use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine as _;

use super::support::*;
use super::support_requests::*;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::tests::{InMemoryBatchServices, InMemoryTriphaseServer};
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage, SafCode, WireAnswer};

const THE_SERVER: &str = "serverUrl=https://sede.example/tri";

fn a_cades_triphase(op: &str) -> AfirmaUrl {
    a_triphase("CAdEStri", op, b"los datos")
}

fn a_triphase(format: &str, op: &str, data: &[u8]) -> AfirmaUrl {
    let text = format!(
        "afirma://{op}?op={op}&idsession={CREDENTIAL}&format={format}&algorithm=SHA256&dat={}",
        URL_SAFE.encode(data)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

fn a_presignature() -> Vec<u8> {
    let xml = format!(
        "<xml>\n <firmas format=\"CAdES\">\n  <firma Id=\"1\">\n   <param n=\"PRE\">{}</param>\n  </firma>\n </firmas>\n</xml>",
        STANDARD.encode(b"prefirma")
    );
    URL_SAFE.encode(xml).into_bytes()
}

fn the_only_row_of(step: ErrandStep) -> String {
    let ErrandStep::AskingToSign(asked) = step else {
        panic!("una firma pide consentimiento: {step:?}");
    };
    asked.certificates[0].id.clone()
}

#[test]
fn a_cades_triphase_signature_hands_the_site_what_the_server_signed() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(THE_SERVER);
    let server = Arc::new(InMemoryTriphaseServer::answering(
        &a_presignature(),
        format!("OK NEWID={}", URL_SAFE.encode(b"la firma del servidor")).as_bytes(),
    ));
    let desk = ErrandDesk {
        triphase: Arc::clone(&server) as _,
        ..a_desk_for_the_batch(
            &engine,
            &policies,
            home.path(),
            &listed,
            &memory,
            &ours,
            Arc::new(InMemoryBatchServices::default()),
        )
    };

    let url = a_cades_triphase("cosign");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    let consented = consent(&desk, &chosen, &live).expect("el certificado sirve");
    assert!(matches!(consented, Consented::SigningWith(_)));
    assert_eq!(
        live.the_certificate_awaiting_the_secret()
            .map(|certificate| certificate.der().to_vec()),
        Some(ours[0].der().to_vec())
    );

    finish_the_server_signature(&desk, "1234", &live).expect("las tres fases salen");
    finish(&desk, &live).expect("la firma se entrega");

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(on_the_wire(&SiteOutcome::Signature {
            signer_der: ours[0].der().to_vec(),
            signature: b"la firma del servidor".to_vec(),
        }))
    );
    assert_eq!(
        desk.neighbours.neighbours.token.signed(),
        vec![("SHA256".to_owned(), b"prefirma".to_vec())]
    );
    let forms = server.forms();
    assert_eq!(forms.len(), 2);
    assert!(forms
        .iter()
        .all(|(_, form)| form.contains(&("cop", "cosign".to_owned()))));
}

#[test]
fn a_triphase_signature_without_server_url_is_refused_with_saf_03_once_the_certificate_is_chosen() {
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
        Arc::new(InMemoryBatchServices::default()),
    );

    let url = a_cades_triphase("sign");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    assert_eq!(what_the_site_received(&mut wire), None);

    let refused = consent(&desk, &chosen, &live).expect_err("no hay servidor al que llamar");

    assert!(matches!(refused, ConsentError::Refused(_)));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::Params).on_the_wire())
    );
}

#[test]
fn a_triphase_server_that_fails_the_presign_is_refused_with_saf_40() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(THE_SERVER);
    let desk = ErrandDesk {
        triphase: Arc::new(InMemoryTriphaseServer::answering(
            b"ERR-14:prefirma:java.io.IOException: la sede no entrega el documento",
            b"",
        )),
        ..a_desk_for_the_batch(
            &engine,
            &policies,
            home.path(),
            &listed,
            &memory,
            &ours,
            Arc::new(InMemoryBatchServices::default()),
        )
    };

    let url = a_cades_triphase("sign");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");

    let refused = finish_the_server_signature(&desk, "1234", &live).expect_err("el servidor fallo");

    assert!(matches!(refused, ConsentError::Refused(_)));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::RecoverServerDocument).on_the_wire())
    );
    assert!(desk.neighbours.neighbours.token.signed().is_empty());
}

fn the_forms_of_a_triphase_signature(
    format: &str,
    data: &[u8],
) -> Vec<Vec<(&'static str, String)>> {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(THE_SERVER);
    let server = Arc::new(InMemoryTriphaseServer::answering(
        &a_presignature(),
        format!("OK NEWID={}", URL_SAFE.encode(b"la firma del servidor")).as_bytes(),
    ));
    let desk = ErrandDesk {
        triphase: Arc::clone(&server) as _,
        ..a_desk_for_the_batch(
            &engine,
            &policies,
            home.path(),
            &listed,
            &memory,
            &ours,
            Arc::new(InMemoryBatchServices::default()),
        )
    };

    let url = a_triphase(format, "sign", data);
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");
    finish_the_server_signature(&desk, "1234", &live).expect("las tres fases salen");
    finish(&desk, &live).expect("la firma se entrega");

    assert_eq!(
        what_the_site_received(&mut wire),
        Some(on_the_wire(&SiteOutcome::Signature {
            signer_der: ours[0].der().to_vec(),
            signature: b"la firma del servidor".to_vec(),
        }))
    );
    server.forms().into_iter().map(|(_, form)| form).collect()
}

#[test]
fn every_triphase_format_reaches_the_server_with_the_format_its_signer_sends() {
    for (format, data, on_the_wire) in [
        ("PAdEStri", &b"%PDF-1.7\n"[..], "pades"),
        ("XAdEStri", &b"<documento/>"[..], "XAdES"),
        ("FacturaEtri", &b"<fe:Facturae/>"[..], "FacturaE"),
    ] {
        let forms = the_forms_of_a_triphase_signature(format, data);

        assert_eq!(forms.len(), 2, "{format}");
        for form in &forms {
            assert!(
                form.contains(&("format", on_the_wire.to_owned())),
                "{format}"
            );
            assert!(form.contains(&("cop", "sign".to_owned())), "{format}");
            assert!(form.contains(&("doc", URL_SAFE.encode(data))), "{format}");
        }
    }
}

fn a_triphase_sign_and_save(format: &str, data: &[u8]) -> AfirmaUrl {
    let text = format!(
        "afirma://signandsave?op=signandsave&cop=sign&idsession={CREDENTIAL}&format={format}&algorithm=SHA256&dat={}",
        URL_SAFE.encode(data)
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

struct SavedThroughTheServer {
    forms: Vec<Vec<(&'static str, String)>>,
    on_disk: Vec<u8>,
    answer: Option<String>,
}

fn saved_through_the_server(url: &AfirmaUrl) -> SavedThroughTheServer {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(THE_SERVER);
    let server = Arc::new(InMemoryTriphaseServer::answering(
        &a_presignature(),
        format!("OK NEWID={}", URL_SAFE.encode(b"la firma del servidor")).as_bytes(),
    ));
    let desk = ErrandDesk {
        triphase: Arc::clone(&server) as _,
        ..a_desk_for_the_batch(
            &engine,
            &policies,
            home.path(),
            &listed,
            &memory,
            &ours,
            Arc::new(InMemoryBatchServices::default()),
        )
    };

    let step = attend_operation(&desk, url, decoded(url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");
    finish_the_server_signature(&desk, "1234", &live).expect("las tres fases salen");
    let Some(ErrandStep::Saving(saving)) = finish(&desk, &live).expect("la firma se guarda") else {
        panic!("signandsave pasa al guardado");
    };
    assert_eq!(what_the_site_received(&mut wire), None);
    assert_eq!(saving.signer_der.as_deref(), Some(ours[0].der()));

    let destination = home.path().join("firma");
    saved(
        &crate::site::adapters::scratch::RealScratch,
        &destination,
        &saving.data,
        saving.signer_der.as_deref(),
        &live,
    );
    SavedThroughTheServer {
        forms: server.forms().into_iter().map(|(_, form)| form).collect(),
        on_disk: std::fs::read(&destination).expect("se ha guardado"),
        answer: what_the_site_received(&mut wire),
    }
}

#[test]
fn signing_and_saving_in_every_triphase_format_saves_and_hands_over_what_the_server_signed() {
    for (format, data, on_the_wire) in [
        ("CAdEStri", &b"los datos"[..], "CAdES"),
        ("PAdEStri", &b"%PDF-1.7\n"[..], "pades"),
        ("XAdEStri", &b"<documento/>"[..], "XAdES"),
        ("FacturaEtri", &b"<fe:Facturae/>"[..], "FacturaE"),
    ] {
        let saved = saved_through_the_server(&a_triphase_sign_and_save(format, data));

        assert_eq!(saved.forms.len(), 2, "{format}");
        for form in &saved.forms {
            assert!(
                form.contains(&("format", on_the_wire.to_owned())),
                "{format}"
            );
            assert!(form.contains(&("cop", "sign".to_owned())), "{format}");
            assert!(form.contains(&("doc", URL_SAFE.encode(data))), "{format}");
        }
        assert_eq!(saved.on_disk, b"la firma del servidor", "{format}");
        let answer = saved.answer.expect("la sede recibe la firma");
        assert!(
            answer.ends_with(&format!("|{}", URL_SAFE.encode(&saved.on_disk))),
            "{format}: {answer}"
        );
    }
}

#[test]
fn signing_and_saving_in_triphase_without_server_url_is_refused_with_saf_03() {
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
        Arc::new(InMemoryBatchServices::default()),
    );

    let url = a_triphase_sign_and_save("CAdEStri", b"los datos");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));

    let refused = consent(&desk, &chosen, &live).expect_err("no hay servidor al que llamar");

    assert!(matches!(refused, ConsentError::Refused(_)));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::Params).on_the_wire())
    );
}

#[test]
fn signing_and_saving_against_a_failing_triphase_server_is_refused_with_saf_40() {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let live = a_live();
    let (handle, mut wire) = the_wire();
    live.answer_through(handle);
    let engine = AnEngine::answering(&[&[0], &[0]]);
    let policies = APolicyEngine::answering(THE_SERVER);
    let desk = ErrandDesk {
        triphase: Arc::new(InMemoryTriphaseServer::answering(
            b"ERR-14:prefirma:java.io.IOException: la sede no entrega el documento",
            b"",
        )),
        ..a_desk_for_the_batch(
            &engine,
            &policies,
            home.path(),
            &listed,
            &memory,
            &ours,
            Arc::new(InMemoryBatchServices::default()),
        )
    };

    let url = a_triphase_sign_and_save("CAdEStri", b"los datos");
    let step = attend_operation(&desk, &url, decoded(&url), &live);
    let chosen = the_only_row_of(remembered(&live, step));
    consent(&desk, &chosen, &live).expect("el certificado sirve");

    let refused = finish_the_server_signature(&desk, "1234", &live).expect_err("el servidor fallo");

    assert!(matches!(refused, ConsentError::Refused(_)));
    assert_eq!(
        what_the_site_received(&mut wire),
        Some(WireAnswer::refused(SafCode::RecoverServerDocument).on_the_wire())
    );
}
