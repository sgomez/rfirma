//! **La ventana de sede y lo que le llega** (ID-333, ID-334, ID-338): el
//! adaptador de Tauri para el trámite, y ninguna decisión dentro.
//!
//! Tres cosas, y las tres son desempaquetar, llamar y traducir (ID-79):
//!
//! - [`open_the_site_window`] crea la ventana. Con qué se abre lo decidió el
//!   arranque ([`crate::app::startup::attend_site_launch`]), que ya apuntó el
//!   momento en el trámite antes de llamar aquí.
//! - [`publish_the_moment`] le cuenta a la ventana, si sigue abierta, en qué
//!   momento está el trámite. El momento lo guarda el trámite
//!   ([`crate::app::errand::LiveErrand::moment`]); aquí sólo se traduce y se
//!   emite.
//! - [`attend_site_operation`] es lo que el transporte llama cuando llega una
//!   operación (ID-330): no es una orden —la llama el canal, no la ventana—
//!   pero hace lo que hace una orden, armar la mesa desde el estado y llamar
//!   al verbo.
//!
//! **El primer momento no se emite, se guarda**: emitirlo al abrir la ventana
//! sería emitirlo al vacío, porque entre que la página carga y que el frontal
//! tiene puesta la escucha van dos idas y vueltas por el IPC. La ventana lo
//! pide al montarse con `read_site_errand`, igual que la invocación con
//! documento pide el suyo con `read_invocation`. Los momentos siguientes sí
//! llegan por el evento (ID-338), que es cuando ya hay quien los oiga.

use tauri::{Emitter as _, Manager as _};

use crate::app::errand::{self, ErrandDesk, ErrandStep, LiveErrand, ReplyHandle};
use crate::app::Environment;
use crate::isolate::Isolate;
use crate::memory::OpenedDocuments;
use crate::protocol::AfirmaUrl;

use super::{SigningSession, SiteErrandView};

/// La etiqueta de la ventana de sede (ID-333).
///
/// Es **suya y sólo suya**: la ventana principal es `main`, y las dos existen a
/// la vez sin que una tape a la otra.
pub const SITE_WINDOW: &str = "site";

/// El nombre del evento con el que la ventana de sede recibe el trámite
/// (ID-338).
///
/// Es un **evento y no un sondeo**: el trámite empuja cada momento nuevo. Que
/// no llegue nunca es la respuesta normal, porque la mayoría de los arranques
/// no vienen de una sede —y entonces esta ventana ni siquiera existe (ID-334)—.
pub const SITE_ERRAND: &str = "site-errand";

/// **La ventana de sede** (ID-333, ID-334): de diálogo, 520 × 420, no
/// redimensionable y sin la cabecera de la aplicación, pero **con las
/// decoraciones del sistema** (`docs/design/ventana-de-sede.md`): la barra de
/// título la pone el escritorio. Una pintada por el frontal no la mueve el
/// gestor de ventanas, y dejaba una ventana que no se podía arrastrar.
///
/// Cerrar por la cruz del sistema es lo mismo que cerrarla por dentro: el
/// manejador de `CloseRequested` de esta etiqueta ya abandona el trámite
/// (ID-340).
///
/// No devuelve nada: si la ventana no se puede crear no hay decisión que tomar,
/// y lo que se hace es contarlo por `stderr`.
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

/// Le publica a la ventana de sede, si sigue abierta, el momento en el que
/// está el trámite.
///
/// Que no esté es una respuesta válida: sin ventana no hay a quien contarle
/// nada, y el trámite no depende de que la haya. Y sin momento tampoco hay
/// nada que contar.
pub fn publish_the_moment(app: &tauri::AppHandle) {
    let Some(moment) = app.state::<LiveErrand>().moment() else {
        return;
    };
    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.emit(SITE_ERRAND, SiteErrandView::from(&moment));
    }
}

/// **La operación de la sede, atendida con la mesa armada desde el estado de
/// la aplicación** (ID-330).
///
/// No es una orden: la llama el transporte, no la ventana. Lo que hace es lo
/// que hace una orden —desempaquetar el estado, llamar al verbo y traducir—, y
/// qué pasa después lo decidió el verbo: el momento del consentimiento se
/// publica hacia la ventana y no escribe nada en el cable; la operación que ya
/// tiene respuesta ya la escribió el trámite al cerrarse (ID-322).
pub fn attend_site_operation(app: &tauri::AppHandle, url: AfirmaUrl, reply: ReplyHandle) {
    let attended = with_the_desk(app, |desk, live| errand::attend(desk, url, reply, live));
    publish_what_moved(app, attended);
}

/// Publica el momento **si el paso dejó uno**: los dos consentimientos y el
/// callejón sin certificado se enseñan; lo que ya está contestado no es un
/// momento nuevo y no mueve la ventana.
pub(super) fn publish_what_moved(app: &tauri::AppHandle, step: Option<ErrandStep>) {
    if step.is_some_and(|step| step.moment().is_some()) {
        publish_the_moment(app);
    }
}

/// Desempaqueta del estado de Tauri todo lo que la mesa del trámite pide, y
/// llama con ella.
///
/// Es la única forma de que la mesa —que presta referencias al estado— viva lo
/// que dura la llamada: los `State<'_, T>` son guardas, y la mesa no puede
/// salir de la función que los tiene.
pub(super) fn with_the_desk<R>(
    app: &tauri::AppHandle,
    call: impl FnOnce(&ErrandDesk<'_, Isolate, Isolate>, &LiveErrand) -> R,
) -> R {
    let environment = app.state::<Environment>();
    let opened = app.state::<OpenedDocuments>();
    let isolate = app.state::<Isolate>();
    let session = app.state::<SigningSession>();
    let live = app.state::<LiveErrand>();
    let desk = ErrandDesk::at(&environment, &opened, &isolate, &session);
    call(&desk, &live)
}
