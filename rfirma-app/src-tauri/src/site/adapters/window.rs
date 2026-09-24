//! Adaptador de la ventana de sede y publicación de eventos del trámite.

use tauri::{Emitter as _, Manager as _};

use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::SigningRoot;
use crate::site::application::errand::{
    self, Acknowledgement, ErrandDesk, ErrandStep, LiveErrand, Moment, ReplyHandle,
};
use crate::site::domain::protocol::{AfirmaUrl, Refusal};
use crate::site::SiteRoot;

use super::desk::Neighbours;
use super::views::SiteErrandView;

/// Etiqueta de la ventana de sede.
pub const SITE_WINDOW: &str = "site";

/// Nombre del evento con el que la ventana de sede recibe el trámite.
pub const SITE_ERRAND: &str = "site-errand";

/// El tamaño de la ventana en todos los momentos menos en el del área.
const DIALOG_SIZE: (f64, f64) = (520.0, 420.0);

/// El tamaño de la ventana mientras la persona marca el área de la firma visible sobre el PDF.
const AREA_SIZE: (f64, f64) = (780.0, 660.0);

/// Tope razonable para esperar a que la respuesta salga por el canal antes de cerrar.
const ERRAND_ENDED_ACKNOWLEDGEMENT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Abre la ventana de diálogo de sede (inicialmente oculta).
pub fn open_the_site_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if app.get_webview_window(SITE_WINDOW).is_some() {
        return;
    }
    let built = WebviewWindowBuilder::new(app, SITE_WINDOW, WebviewUrl::App("sede.html".into()))
        .title("rFirma")
        .inner_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
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

/// Contesta al trámite vivo, si lo hay, antes de dejar cerrar la ventana de sede por el gestor de
/// ventanas: retiene el cierre, contesta el rechazo que enseñaba o cancela, y reintenta cerrar
/// cuando termina, momento en el que este mismo evento vuelve a llegar con el trámite ya terminado.
/// Mientras el WebSocket siga sirviendo, la oculta en vez de cerrarla (ADR-0024); con el diálogo
/// del área abierto, solo lo cancela.
fn handle_close_requested(app: &tauri::AppHandle, event: &tauri::WindowEvent) {
    let tauri::WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let live = &app.state::<SiteRoot>().errand;
    if live.current().is_none() && !live.holds_back_a_launch() {
        return;
    }
    api.prevent_close();
    let app = app.clone();
    std::thread::spawn(move || {
        let after = errand::answer_before_closing(&app.state::<SiteRoot>().errand);
        if after != errand::WindowAfterClosing::Closes {
            publish_the_moment(&app);
            return;
        }
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

    fn hide(&self) {
        if let Some(window) = self.app.get_webview_window(SITE_WINDOW) {
            let _ = window.hide();
        }
    }

    fn close(&self) {
        if let Some(window) = self.app.get_webview_window(SITE_WINDOW) {
            let _ = window.close();
        }
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
        fit_to(&window, &moment);
        let _ = window.emit(SITE_ERRAND, SiteErrandView::from(&moment));
    }
}

/// Agranda la ventana para el área de la firma visible y la devuelve a su tamaño al salir.
fn fit_to(window: &tauri::WebviewWindow, moment: &Moment) {
    let (width, height) = if matches!(moment, Moment::MarkingTheArea { .. }) {
        AREA_SIZE
    } else {
        DIALOG_SIZE
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    let current = window
        .inner_size()
        .map(|size| size.to_logical::<f64>(scale));
    if current.is_ok_and(|size| size.width.round() == width && size.height.round() == height) {
        return;
    }
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
    let _ = window.center();
}

/// Publica el rechazo de un servidor intermedio que no pudo entregar la respuesta a la sede.
pub fn note_a_relay_failure(app: &tauri::AppHandle, refusal: Refusal) {
    app.state::<SiteRoot>()
        .errand
        .the_site_did_not_get_the_answer(refusal);
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
        triphase: site.triphase.clone(),
    };
    call(&desk, &site.errand)
}
