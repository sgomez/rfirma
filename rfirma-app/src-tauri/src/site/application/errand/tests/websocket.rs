//! Pruebas del tramite de sede sobre WebSocket.

use std::cell::RefCell;
use std::sync::Arc;

use crate::site::application::errand::*;
use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::signing::application::tests::a_memory;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::application::startup::{SiteWindow, SiteWindowContent};
use crate::site::domain::channel::{
    ArrivalMode, ChannelTenure,
};
use crate::site::domain::protocol::{
    AfirmaUrl, ChannelMessage, NegotiatedCredential,
    SafCode,
};
use super::support::*;

/// Ventana doblada que apunta lo que el trámite le pide.
#[derive(Default)]
struct AWindow {
    asked: std::sync::Mutex<Vec<&'static str>>,
}

impl AWindow {
    fn asked(&self) -> Vec<&'static str> {
        self.asked
            .lock()
            .expect("el doble no envenena su cerrojo")
            .clone()
    }

    fn note(&self, what: &'static str) {
        self.asked
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push(what);
    }
}

impl SiteWindow for AWindow {
    fn open(&self, _content: SiteWindowContent<'_>) {
        self.note("abierta");
    }
    fn show(&self) {
        self.note("enseñada");
    }
    fn hide(&self) {
        self.note("oculta");
    }
    fn close(&self) {
        self.note("cerrada");
    }
    fn errand_ended(&self, _delivered: Acknowledgement) {
        self.note("trámite-terminado");
    }
}

fn a_websocket_errand_begun() -> LiveErrand {
    let live = LiveErrand::default();
    assert!(live.begin(
        Errand::of(
            NegotiatedCredential::Required(a_credential()),
            ArrivalMode::Awaited,
            a_codec(),
        )
        .with_tenure(ChannelTenure::WhileTheFirstClientStays)
    ));
    live
}

/// Un trámite de WebSocket vivo, con su ventana, y el navegador ya llegado.
fn a_websocket_errand(window: &Arc<AWindow>) -> LiveErrand {
    let live = a_websocket_errand_begun();
    live.keep_the_window(Arc::clone(window) as Arc<dyn SiteWindow>);
    live.browser_arrived();
    live
}

fn arriving(message: &str) -> AfirmaUrl {
    let ChannelMessage::Operation { url } = ChannelMessage::read(message) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn a_websocket_launch_begins_an_errand_that_serves_many_operations() {
    let asked = RefCell::new(Vec::new());
    let live = LiveErrand::default();

    let attendance = attend_launch(
        &a_launch("54001,54002,54003"),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );

    let Attendance::Serving { errand, .. } = attendance else {
        panic!("un arranque bueno se atiende: {attendance:?}");
    };
    assert_eq!(errand.tenure(), ChannelTenure::WhileTheFirstClientStays);
}

#[test]
fn a_websocket_errand_answers_the_operation_after_the_refusal_it_showed() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );

    let (first, mut first_wire) = the_wire();
    let refused = attend(
        &desk,
        arriving(&format!("afirma://unknownop?idsession={CREDENTIAL}")),
        first,
        &live,
    );
    let _ = answer_before_closing_within(&live, Duration::from_secs(1));
    let (second, mut second_wire) = the_wire();
    let answered = attend(&desk, an_operation(""), second, &live);

    assert!(matches!(refused, Some(ErrandStep::ShowingTheRefusal(_))));
    assert!(what_the_site_received(&mut first_wire).is_some_and(|line| line.starts_with("SAF_04")));
    assert!(matches!(answered, Some(ErrandStep::Answering(_))));
    assert!(
        what_the_site_received(&mut second_wire).is_some(),
        "la segunda operacion tambien se contesta"
    );
    assert!(
        live.current().is_some(),
        "el tramite sigue mientras siga el primer cliente"
    );
    assert!(
        !window.asked().contains(&"trámite-terminado"),
        "la ventana no se entera de un final que no lo es: {:?}",
        window.asked()
    );
}

#[test]
fn a_websocket_operation_shows_the_window_and_hides_it_when_it_answers_unseen() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );
    live.note(Moment::Unreachable);

    let (handle, _wire) = the_wire();
    let _ = attend(&desk, an_operation(""), handle, &live);

    assert_eq!(window.asked(), ["enseñada", "oculta"]);
    assert_eq!(
        live.moment(),
        Some(Moment::Waiting),
        "la siguiente operacion se enseña como la primera"
    );
}

/// Asa de respuesta que apunta en la ventana doblada cuándo sale la respuesta.
fn a_wire_noting_into(
    window: &Arc<AWindow>,
) -> (ReplyHandle, tokio::sync::oneshot::Receiver<String>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let window = Arc::clone(window);
    (
        ReplyHandle::of(move |text| {
            window.note("contestada");
            let _ = sender.send(text);
            Acknowledgement::immediate()
        }),
        receiver,
    )
}

/// Atiende la operación sobre una mesa sin certificados ni motores que contesten.
fn attended_on_a_bare_desk(url: AfirmaUrl, reply: ReplyHandle, live: &LiveErrand) -> ErrandStep {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );
    attend(&desk, url, reply, live).expect("hay codec")
}

#[test]
fn a_refusal_of_the_request_is_shown_and_answered_only_once_its_window_closes() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    let (handle, mut wire) = a_wire_noting_into(&window);

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=CAdES&algorithm=SHA256withRSA&dat=SG9sYQ&id=rfirma-1&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::ShowingTheRefusal(_)), "{step:?}");
    assert!(
        matches!(live.moment(), Some(Moment::ShowingTheRefusal(ref refusal)) if refusal.code() == SafCode::Params),
        "{:?}",
        live.moment()
    );
    assert_eq!(
        what_the_site_received(&mut wire),
        None,
        "la sede espera a que se cierre"
    );
    assert_eq!(window.asked(), ["enseñada"]);

    let after = answer_before_closing_within(&live, Duration::from_secs(1));

    assert_eq!(after, WindowAfterClosing::StaysHidden);
    assert!(what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_03")));
    assert_eq!(
        window.asked(),
        ["enseñada", "oculta", "contestada"],
        "se oculta antes de contestar, o la siguiente operación podría llegar a una ventana que se oculta después"
    );
    assert!(live.current().is_some(), "el canal sigue sirviendo");
}

#[test]
fn a_refusal_the_original_answers_without_a_dialogue_is_answered_at_once() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    let (handle, mut wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=INVENTADO&algorithm=SHA256&dat=SG9sYQ&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::Answering(_)), "{step:?}");
    assert!(what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_06")));
    assert_eq!(window.asked(), ["enseñada", "oculta"]);
}

#[test]
fn cosigning_an_invoice_is_answered_at_once_as_the_original_answers_it_while_signing() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    let (handle, mut wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://cosign?op=cosign&format=FacturaE&algorithm=SHA256withRSA&dat=PEZhY3R1cmFlPjxGaWxlSGVhZGVyLz48UGFydGllcy8-PEludm9pY2VzLz48L0ZhY3R1cmFlPg==&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::Answering(_)), "{step:?}");
    assert!(what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_04")));
}

#[test]
fn closing_the_window_of_a_shown_refusal_answers_it_instead_of_cancelling() {
    let live = LiveErrand::default();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        Arc::new(ACodec::answering(vec![SiteRequest::NotAttended(
            crate::site::domain::protocol::Refusal::params("el id no es alfanumerico"),
        )])),
    )));
    let (handle, mut wire) = the_wire();
    let _ = attended_on_a_bare_desk(an_operation(""), handle, &live);

    let after = answer_before_closing_within(&live, Duration::from_secs(1));

    assert_eq!(after, WindowAfterClosing::Closes);
    assert!(
        what_the_site_received(&mut wire)
            .is_some_and(|line| line.starts_with("RefusedByTheProtocol(")),
        "la sede recibe el rechazo, no un CANCEL"
    );
    assert!(live.current().is_none());
}

#[test]
fn the_first_client_leaving_ends_the_websocket_errand_and_closes_its_window() {
    let window = Arc::new(AWindow::default());
    let live = a_websocket_errand(&window);
    assert!(live.keeps_serving());

    live.the_first_client_left();

    assert!(live.current().is_none());
    assert!(!live.keeps_serving());
    assert_eq!(window.asked(), ["cerrada"]);
}

#[test]
fn a_websocket_errand_keeps_serving_only_once_the_browser_has_arrived() {
    let live = a_websocket_errand_begun();
    assert!(
        !live.keeps_serving(),
        "sin navegador, cerrar la ventana termina como siempre"
    );

    live.browser_arrived();

    assert!(live.keeps_serving());
}

#[test]
fn a_single_operation_errand_ignores_the_first_client_leaving_and_never_keeps_serving() {
    let window = Arc::new(AWindow::default());
    let live = LiveErrand::default();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec(),
    )));
    live.keep_the_window(Arc::clone(&window) as Arc<dyn SiteWindow>);
    live.browser_arrived();

    live.the_first_client_left();

    assert!(live.current().is_some());
    assert!(!live.keeps_serving());
    assert!(window.asked().is_empty());
}
