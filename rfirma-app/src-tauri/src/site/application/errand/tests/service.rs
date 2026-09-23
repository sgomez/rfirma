//! Pruebas del tramite de sede sobre el canal `service`.

use std::cell::RefCell;
use std::sync::Arc;

use super::support::*;
use super::support_window::*;
use crate::site::application::errand::*;
use crate::site::application::site::{attend_launch, Attendance};
use crate::site::application::startup::SiteWindow;
use crate::site::domain::channel::ChannelTenure;
use crate::site::domain::protocol::{AfirmaUrl, ChannelMessage};

fn a_service_launch() -> String {
    format!("afirma://service?ports=54351,54352,54353&v=3&jvc=3&idsession={CREDENTIAL}")
}

/// Un trámite de `service` recién arrancado, con su ventana, y el navegador ya llegado.
fn a_service_errand(window: &Arc<AWindow>) -> LiveErrand {
    let asked = RefCell::new(Vec::new());
    let live = LiveErrand::default();
    let attendance = attend_launch(
        &a_service_launch(),
        &a_codec_table(),
        &a_transport(&asked),
        &live,
    );
    assert!(
        matches!(attendance, Attendance::Serving { .. }),
        "{attendance:?}"
    );
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
fn a_service_launch_begins_an_errand_that_serves_until_its_channel_idles() {
    let window = Arc::new(AWindow::default());
    let live = a_service_errand(&window);

    assert_eq!(
        live.current().map(|errand| errand.tenure()),
        Some(ChannelTenure::UntilTheChannelIdles)
    );
    assert!(live.keeps_serving());
}

#[test]
fn closing_a_shown_refusal_answers_it_and_keeps_the_service_channel_serving() {
    let window = Arc::new(AWindow::default());
    let live = a_service_errand(&window);
    let (handle, mut wire) = the_wire();

    let step = attended_on_a_bare_desk(
        arriving(&format!("afirma://sign?op=sign&format=NoSuchFormat&algorithm=SHA256withRSA&dat=file:/etc/hostname&idsession={CREDENTIAL}")),
        handle,
        &live,
    );
    assert!(matches!(step, ErrandStep::ShowingTheRefusal(_)), "{step:?}");

    let after = answer_before_closing_within(&live, Duration::from_secs(1));

    assert_eq!(
        after,
        WindowAfterClosing::StaysHidden,
        "cerrar el rechazo no puede cerrar la ventana, o el proceso y el canal mueren con ella"
    );
    assert!(what_the_site_received(&mut wire).is_some_and(|line| line.starts_with("SAF_03")));
    assert!(live.current().is_some(), "el canal sigue sirviendo");
}

#[test]
fn the_channel_going_idle_ends_the_service_errand_and_closes_its_window() {
    let window = Arc::new(AWindow::default());
    let live = a_service_errand(&window);

    live.the_channel_went_idle();

    assert!(live.current().is_none());
    assert_eq!(window.asked(), ["cerrada"]);
}
