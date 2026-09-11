//! Adaptador de la ventana de sede y publicación de eventos del trámite.

use tauri::{Emitter as _, Manager as _};

use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::SigningRoot;
use crate::site::application::errand::{
    self, Acknowledgement, ErrandDesk, ErrandStep, LiveErrand, ReplyHandle,
};
use crate::site::domain::protocol::{AfirmaUrl, Refusal};
use crate::site::SiteRoot;

use super::desk::Neighbours;
use super::views::SiteErrandView;

/// Etiqueta de la ventana de sede.
pub const SITE_WINDOW: &str = "site";

/// Nombre del evento con el que la ventana de sede recibe el trámite.
pub const SITE_ERRAND: &str = "site-errand";

/// Tope razonable para esperar a que la respuesta salga por el canal antes de cerrar.
const ERRAND_ENDED_ACKNOWLEDGEMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Abre la ventana de diálogo de sede (inicialmente oculta).
pub fn open_the_site_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    let built = WebviewWindowBuilder::new(app, SITE_WINDOW, WebviewUrl::App("sede.html".into()))
        .title("rFirma")
        .inner_size(520.0, 420.0)
        .resizable(false)
        .visible(false)
        .build();

    match built {
        Ok(window) => {
            let app = app.clone();
            window.on_window_event(move |event| handle_close_requested(&app, event));
        }
        Err(error) => eprintln!("rfirma: no se puede abrir la ventana de sede ({error})"),
    }
}

/// Cancela el trámite vivo, si lo hay, antes de dejar cerrar la ventana de sede por el gestor de
/// ventanas: retiene el cierre, cancela como el botón de cancelar y reintenta cerrar cuando
/// termina, momento en el que este mismo evento vuelve a llegar con el trámite ya terminado.
fn handle_close_requested(app: &tauri::AppHandle, event: &tauri::WindowEvent) {
    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    if app.state::<SiteRoot>().errand.current().is_none() {
        return;
    }
    api.prevent_close();
    let app = app.clone();
    std::thread::spawn(move || {
        errand::decline_before_closing(&app.state::<SiteRoot>().errand);
        if let Some(window) = app.get_webview_window(SITE_WINDOW) {
            let _ = window.close();
        }
    });
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

    fn errand_ended(&self, delivered: Acknowledgement) {
        delivered.wait(ERRAND_ENDED_ACKNOWLEDGEMENT_TIMEOUT);
        let Some(window) = self.app.get_webview_window(SITE_WINDOW) else {
            return;
        };
        if !window.is_visible().unwrap_or(true) {
            let _ = window.close();
        }
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
        validation: &signing.isolate,
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
