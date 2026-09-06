//! Adaptador de la ventana de sede y publicación de eventos del trámite.

use tauri::{Emitter as _, Manager as _};

use crate::documents::application::opened::OpenedDocuments;
use crate::signing::adapters::isolate::Isolate;
use crate::site::application::errand::{self, ErrandDesk, ErrandStep, LiveErrand, ReplyHandle};
use crate::site::domain::protocol::AfirmaUrl;
use crate::Environment;

use super::views::SiteErrandView;
use crate::signing::application::session::SigningSession;

/// Etiqueta de la ventana de sede.
pub const SITE_WINDOW: &str = "site";

/// Nombre del evento con el que la ventana de sede recibe el trámite.
pub const SITE_ERRAND: &str = "site-errand";

/// Abre la ventana de diálogo de sede.
pub fn open_the_site_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let built = WebviewWindowBuilder::new(app, SITE_WINDOW, WebviewUrl::App("sede.html".into()))
        .title("rFirma")
        .inner_size(520.0, 420.0)
        .resizable(false)
        .build();

    if let Err(error) = built {
        eprintln!("rfirma: no se puede abrir la ventana de sede ({error})");
    }
}

/// Publica a la ventana de sede el momento actual del trámite.
pub fn publish_the_moment(app: &tauri::AppHandle) {
    let Some(moment) = app.state::<LiveErrand>().moment() else {
        return;
    };
    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.emit(SITE_ERRAND, SiteErrandView::from(&moment));
    }
}

/// Atiende una operación de sede armando la mesa desde el estado de la aplicación.
pub fn attend_site_operation(app: &tauri::AppHandle, url: AfirmaUrl, reply: ReplyHandle) {
    let attended = with_the_desk(app, |desk, live| errand::attend(desk, url, reply, live));
    publish_what_moved(app, attended);
}

/// Publica el momento si el paso del trámite produjo uno nuevo.
pub(super) fn publish_what_moved(app: &tauri::AppHandle, step: Option<ErrandStep>) {
    if step.is_some_and(|step| step.moment().is_some()) {
        publish_the_moment(app);
    }
}

/// Desempaqueta del estado de Tauri los componentes de la mesa del trámite.
pub(super) fn with_the_desk<R>(
    app: &tauri::AppHandle,
    call: impl FnOnce(&ErrandDesk<'_, Isolate, Isolate, Isolate>, &LiveErrand) -> R,
) -> R {
    let environment = app.state::<Environment>();
    let opened = app.state::<OpenedDocuments>();
    let isolate = app.state::<Isolate>();
    let session = app.state::<SigningSession>();
    let live = app.state::<LiveErrand>();
    let desk = ErrandDesk::at(&environment, &opened, &isolate, &session);
    call(&desk, &live)
}
