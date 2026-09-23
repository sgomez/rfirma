//! Pruebas de cuándo enseña su ventana un trámite de llegada inmediata, el del servidor intermedio.

use std::sync::Arc;

use super::support::*;
use crate::site::application::errand::*;
use crate::site::application::startup::SiteWindow;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{ChannelMessage, NegotiatedCredential};

fn an_immediate_errand_with(window: &Arc<AWindow>) -> LiveErrand {
    let live = LiveErrand::default();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Immediate,
        a_codec(),
    )));
    live.keep_the_window(Arc::clone(window) as Arc<dyn SiteWindow>);
    live
}

fn arriving(message: &str) -> crate::site::domain::protocol::AfirmaUrl {
    let ChannelMessage::Operation { url } = ChannelMessage::read(message) else {
        panic!("una URL del protocolo es una operacion");
    };
    url
}

#[test]
fn an_immediate_operation_answered_at_once_ends_the_errand_without_showing_its_window() {
    let window = Arc::new(AWindow::default());
    let live = an_immediate_errand_with(&window);
    let (handle, mut wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256&dat=SG9sYQ&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::Answering(_)), "{step:?}");
    assert!(what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_06")));
    assert_eq!(window.asked(), ["trámite-terminado"]);
}

#[test]
fn an_immediate_operation_with_something_to_say_shows_its_window() {
    let window = Arc::new(AWindow::default());
    let live = an_immediate_errand_with(&window);
    let (handle, mut wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=CAdES&algorithm=SHA256withRSA&dat=SG9sYQ&id=rfirma-1&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::ShowingTheRefusal(_)), "{step:?}");
    assert_eq!(what_the_site_received(&mut wire), None);
    assert_eq!(window.asked(), ["enseñada"]);
}

#[test]
fn an_awaited_operation_leaves_showing_its_window_to_the_browser_arrival() {
    let window = Arc::new(AWindow::default());
    let live = LiveErrand::default();
    assert!(live.begin(Errand::of(
        NegotiatedCredential::Required(a_credential()),
        ArrivalMode::Awaited,
        a_codec(),
    )));
    live.keep_the_window(Arc::clone(&window) as Arc<dyn SiteWindow>);
    let (handle, _wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=CAdES&algorithm=SHA256withRSA&dat=SG9sYQ&id=rfirma-1&idsession={CREDENTIAL}")),
        handle,
        &live,
    );

    assert!(matches!(step, ErrandStep::ShowingTheRefusal(_)), "{step:?}");
    assert!(window.asked().is_empty());
}
