//! Pruebas de los rechazos deliberados de la firma de sede, que se enseñan antes de contestar.

use std::path::Path;
use std::time::Duration;

use super::support::*;
use super::support_requests::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::tests::{a_usable_certificate, listed_from};
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, NegotiatedCredential, RefusalSituation, SafCode, WireAnswer,
};
use base64::Engine as _;

const AN_INVOICE: &[u8] = b"<Facturae><FileHeader/><Parties/><Invoices/></Facturae>";

fn a_request(verb: &str, format: &str, document: Option<&[u8]>, properties: &str) -> AfirmaUrl {
    let base64 = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE.encode(bytes);
    let dat = document.map_or(String::new(), |bytes| format!("&dat={}", base64(bytes)));
    let text = format!(
        "afirma://{verb}?op={verb}&idsession={CREDENTIAL}&format={format}&\
         algorithm=SHA256withRSA&properties={}{dat}",
        base64(properties.as_bytes())
    );
    let ChannelMessage::Operation { url } = ChannelMessage::read(&text) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

struct Refused {
    step: ErrandStep,
    shown: Option<Moment>,
    received_before_closing: Option<String>,
    received_after_closing: Option<String>,
}

/// Atiende `url`, eligiendo `chosen` en disco si la petición no trae `dat`, y cierra la ventana.
fn what_happens_with(url: AfirmaUrl, chosen: Option<&[u8]>) -> Refused {
    let home = tempfile::tempdir().expect("deberia haber directorio temporal");
    let memory = a_memory(home.path());
    let ours = vec![a_usable_certificate("FIRMA")];
    let (listed, _) = listed_from(&ours);
    let opened = OpenedDocuments::new();
    let engine = AnEngine::answering(&[&[0]]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let mut desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened,
        &memory,
        &scratch,
    );
    desk.neighbours.ours = ours;
    let live = a_live();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec()
    )));
    let (handle, mut wire) = the_wire();

    let mut step = attend(&desk, url, handle, &live).expect("hay codec");
    if let (ErrandStep::Loading(_), Some(document)) = (&step, chosen) {
        step = continued_with(&desk, home.path(), document, &live);
    }
    let shown = live.moment();
    let received_before_closing = what_the_site_received(&mut wire);
    answer_before_closing_within(&live, Duration::from_millis(50));
    Refused {
        step,
        shown,
        received_before_closing,
        received_after_closing: what_the_site_received(&mut wire),
    }
}

fn continued_with<N: Neighbours>(
    desk: &ErrandDesk<'_, AnEngine, APolicyEngine, N>,
    home: &Path,
    document: &[u8],
    live: &LiveErrand,
) -> ErrandStep {
    let path = home.join("elegido");
    std::fs::write(&path, document).expect("se escribe el fichero elegido");
    let LoadCompletion::Continues(step) =
        document_chosen(desk, &[("elegido".to_owned(), path)], live)
    else {
        panic!("el documento elegido se lee y el tramite sigue");
    };
    step
}

fn assert_shown_then_answered(refused: &Refused, situation: RefusalSituation, code: SafCode) {
    let ErrandStep::ShowingTheRefusal(refusal) = &refused.step else {
        panic!("el rechazo se enseña: {:?}", refused.step);
    };
    assert_eq!(refusal.situation(), situation);
    assert!(
        matches!(&refused.shown, Some(Moment::ShowingTheRefusal(shown)) if shown.situation() == situation),
        "{:?}",
        refused.shown
    );
    assert_eq!(
        refused.received_before_closing, None,
        "hasta que se cierre la ventana"
    );
    assert_eq!(
        refused.received_after_closing,
        Some(WireAnswer::refused(code).on_the_wire())
    );
}

#[test]
fn an_explicit_xades_sent_by_the_site_is_shown_before_the_site_gets_saf_06() {
    let url = a_request("sign", "XAdES", Some(AN_XML_CHALLENGE), "mode=explicit\n");

    let refused = what_happens_with(url, None);

    assert_shown_then_answered(
        &refused,
        RefusalSituation::ExplicitXades,
        SafCode::UnsupportedFormat,
    );
}

#[test]
fn an_explicit_xades_over_a_chosen_document_is_shown_instead_of_leaving_the_selector_hanging() {
    let url = a_request("sign", "XAdES", None, "mode=explicit\n");

    let refused = what_happens_with(url, Some(AN_XML_CHALLENGE));

    assert_shown_then_answered(
        &refused,
        RefusalSituation::ExplicitXades,
        SafCode::UnsupportedFormat,
    );
}

#[test]
fn a_cosignature_of_an_invoice_sent_by_the_site_is_shown_before_the_site_gets_saf_04() {
    let url = a_request("cosign", "FacturaE", Some(AN_INVOICE), "");

    let refused = what_happens_with(url, None);

    assert_shown_then_answered(
        &refused,
        RefusalSituation::InvoiceMultisignature,
        SafCode::UnsupportedOperation,
    );
}

#[test]
fn a_cosignature_of_a_chosen_invoice_is_shown_instead_of_leaving_the_selector_hanging() {
    let url = a_request("cosign", "auto", None, "");

    let refused = what_happens_with(url, Some(AN_INVOICE));

    assert_shown_then_answered(
        &refused,
        RefusalSituation::InvoiceMultisignature,
        SafCode::UnsupportedOperation,
    );
}

#[test]
fn a_countersignature_of_a_pdf_sent_by_the_site_is_shown_before_the_site_gets_saf_04() {
    let url = a_request("countersign", "PAdES", Some(A_PDF), "");

    let refused = what_happens_with(url, None);

    assert_shown_then_answered(
        &refused,
        RefusalSituation::UnsupportedCountersignature,
        SafCode::UnsupportedOperation,
    );
}

#[test]
fn a_countersignature_of_a_chosen_pdf_is_shown_instead_of_leaving_the_selector_hanging() {
    let url = a_request("countersign", "auto", None, "");

    let refused = what_happens_with(url, Some(A_PDF));

    assert_shown_then_answered(
        &refused,
        RefusalSituation::UnsupportedCountersignature,
        SafCode::UnsupportedOperation,
    );
}
