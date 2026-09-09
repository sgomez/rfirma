//! Adaptador de la ventana de sede y publicación de eventos del trámite.

use tauri::{Emitter as _, Manager as _};

use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::SigningRoot;
use crate::site::application::errand::{self, ErrandDesk, ErrandStep, LiveErrand, ReplyHandle};
use crate::site::domain::protocol::{AfirmaUrl, Refusal};
use crate::site::SiteRoot;

use super::desk::Neighbours;
use super::views::SiteErrandView;

/// Etiqueta de la ventana de sede.
pub const SITE_WINDOW: &str = "site";

/// Nombre del evento con el que la ventana de sede recibe el trámite.
pub const SITE_ERRAND: &str = "site-errand";

/// Abre la ventana de diálogo de sede (inicialmente oculta).
pub fn open_the_site_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let built = WebviewWindowBuilder::new(app, SITE_WINDOW, WebviewUrl::App("sede.html".into()))
        .title("rFirma")
        .inner_size(520.0, 420.0)
        .resizable(false)
        .visible(false)
        .build();

    if let Err(error) = built {
        eprintln!("rfirma: no se puede abrir la ventana de sede ({error})");
    }
}

/// Muestra y da foco a la ventana de diálogo de sede.
pub fn show_the_site_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Puerto de ventana para la aplicación Tauri.
#[derive(Clone)]
pub struct TauriSiteWindow {
    app: tauri::AppHandle,
}

impl TauriSiteWindow {
    /// Crea un puerto de ventana vinculado al manejador de Tauri.
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl crate::site::application::startup::SiteWindow for TauriSiteWindow {
    fn open(&self, _content: crate::site::application::startup::SiteWindowContent<'_>) {
        open_the_site_window(&self.app);
    }

    fn show(&self) {
        publish_the_moment(&self.app);
        show_the_site_window(&self.app);
    }
}

/// Publica a la ventana de sede el momento actual del trámite.
pub fn publish_the_moment(app: &tauri::AppHandle) {
    let Some(moment) = app.state::<SiteRoot>().errand.moment() else {
        return;
    };
    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.emit(SITE_ERRAND, SiteErrandView::from(&moment));
    }
}

/// Publica el rechazo de un servidor intermedio que no pudo entregar la respuesta a la sede.
pub fn note_a_relay_failure(app: &tauri::AppHandle, refusal: Refusal) {
    app.state::<SiteRoot>()
        .errand
        .note(errand::Moment::RefusedWithoutChannel(refusal));
    publish_the_moment(app);
}

/// Atiende una operación de sede armando la mesa desde el estado de la aplicación.
pub fn attend_site_operation(app: &tauri::AppHandle, url: AfirmaUrl, reply: ReplyHandle) {
    super::trace::note_the_operation(&url);
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
pub(crate) fn with_the_desk<R>(
    app: &tauri::AppHandle,
    call: impl FnOnce(&ErrandDesk<'_, Isolate, Isolate, Neighbours<'_>>, &LiveErrand) -> R,
) -> R {
    let identity = app.state::<IdentityRoot>();
    let documents = app.state::<DocumentsRoot>();
    let signing = app.state::<SigningRoot>();
    let site = app.state::<SiteRoot>();
    let desk = ErrandDesk {
        engine: &signing.isolate,
        policies: &signing.isolate,
        neighbours: Neighbours {
            identity: &identity,
            documents: &documents,
            signing: &signing,
        },
        scratch_dir: site.scratch_dir.clone(),
        scratch: site.scratch.clone(),
        batch: site.batch.clone(),
    };
    call(&desk, &site.errand)
}
