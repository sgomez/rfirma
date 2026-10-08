//! Adaptadores del puerto `SecretPrompter`: diálogo nativo GTK3, sus pendientes de Windows y macOS y los de pruebas (ADR-0001, ADR-0014, ADR-0035, ADR-0040).

use std::sync::Mutex;
#[cfg(target_os = "linux")]
use std::sync::{Arc, OnceLock};

use crate::identity::domain::protected_secret::ProtectedSecret;
#[cfg(target_os = "linux")]
use crate::identity::domain::secret::PinWarning;
#[cfg(target_os = "linux")]
use crate::identity::ports::OriginWindow;
use crate::identity::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};

mod texts;

pub use texts::{localize, DialogI18n};

#[cfg(target_os = "linux")]
/// Adaptador de producción que presenta un diálogo modal nativo GTK3 para la solicitud de PIN.
#[derive(Clone, Default)]
pub struct GtkSecretPrompter {
    app: Arc<OnceLock<tauri::AppHandle>>,
}

#[cfg(target_os = "linux")]
impl GtkSecretPrompter {
    /// Crea un adaptador vacío pendiente de vincular al manejador de Tauri.
    pub fn new() -> Self {
        Self::default()
    }

    /// Vincula el manejador de la aplicación al adaptador, tras montarse la ventana (ADR-0024).
    pub fn attach(&self, app: tauri::AppHandle) {
        let _ = self.app.set(app);
    }

    /// La ventana que pidió el secreto, si se conoce y sigue montada.
    fn window_to_be_modal_over(
        &self,
        origin: Option<OriginWindow>,
    ) -> Option<gtk::ApplicationWindow> {
        use tauri::Manager as _;

        let app = self.app.get()?;
        app.get_webview_window(origin?.label())?.gtk_window().ok()
    }
}

#[cfg(target_os = "linux")]
fn heading_of(i18n: &DialogI18n) -> gtk::Box {
    use gtk::prelude::*;

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);

    let icon = gtk::Image::from_icon_name(Some("dialog-password"), gtk::IconSize::Dialog);
    icon.set_valign(gtk::Align::Start);
    heading.pack_start(&icon, false, false, 0);

    let lines = gtk::Box::new(gtk::Orientation::Vertical, 2);

    let primary = gtk::Label::new(None);
    primary.set_markup(&format!(
        "<b>{}</b>",
        glib::markup_escape_text(i18n.holder_name.as_deref().unwrap_or(&i18n.title))
    ));
    primary.set_halign(gtk::Align::Start);
    primary.set_xalign(0.0);
    primary.set_line_wrap(true);
    lines.pack_start(&primary, false, false, 0);

    if let Some(id_number) = &i18n.id_number {
        let secondary = gtk::Label::new(Some(id_number));
        secondary.set_halign(gtk::Align::Start);
        secondary.set_xalign(0.0);
        secondary.set_line_wrap(true);
        secondary.style_context().add_class("dim-label");
        lines.pack_start(&secondary, false, false, 0);
    }

    heading.pack_start(&lines, true, true, 0);
    heading
}

#[cfg(target_os = "linux")]
fn dialog_for(i18n: &DialogI18n, parent: Option<gtk::ApplicationWindow>) -> gtk::Dialog {
    use gtk::prelude::*;

    let dialog = gtk::Dialog::builder()
        .title(i18n.title.as_str())
        .modal(true)
        .resizable(false)
        .icon_name("dialog-password")
        .build();
    dialog.set_default_size(400, -1);

    match parent {
        Some(parent) => {
            dialog.set_transient_for(Some(&parent));
            dialog.set_destroy_with_parent(true);
            dialog.set_position(gtk::WindowPosition::CenterOnParent);
        }
        None => dialog.set_position(gtk::WindowPosition::Center),
    }

    dialog.add_button(i18n.cancel, gtk::ResponseType::Cancel);
    let accept = dialog.add_button(i18n.accept, gtk::ResponseType::Ok);
    accept.style_context().add_class("suggested-action");
    dialog.set_default_response(gtk::ResponseType::Ok);

    dialog
}

#[cfg(target_os = "linux")]
fn masked_entry() -> gtk::Entry {
    use gtk::prelude::*;

    let entry = gtk::Entry::new();
    entry.set_visibility(false);
    entry.set_input_purpose(gtk::InputPurpose::Password);
    entry.set_invisible_char(Some('•'));
    entry.set_activates_default(true);
    entry.set_width_chars(24);
    entry
}

#[cfg(target_os = "linux")]
fn failure_label(text: &str) -> gtk::Label {
    use gtk::prelude::*;

    let failure = gtk::Label::new(None);
    failure.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(text)));
    failure.set_halign(gtk::Align::Start);
    failure.set_xalign(0.0);
    failure.set_line_wrap(true);
    failure
}

#[cfg(target_os = "linux")]
fn say_the_previous_attempt_was_wrong(content_area: &gtk::Box, entry: &gtk::Entry, text: &str) {
    use gtk::prelude::*;

    entry.style_context().add_class("error");
    content_area.pack_start(&failure_label(text), false, false, 0);
}

#[cfg(target_os = "linux")]
fn warning_label(text: &str, warning: PinWarning) -> gtk::Label {
    use gtk::prelude::*;

    let label = gtk::Label::new(None);
    let escaped = glib::markup_escape_text(text);
    match warning {
        PinWarning::FinalTry => label.set_markup(&format!("<b>{escaped}</b>")),
        PinWarning::CountLow | PinWarning::Quiet => label.set_markup(&escaped),
    }
    label.set_halign(gtk::Align::Start);
    label.set_xalign(0.0);
    label.set_line_wrap(true);
    label
}

#[cfg(target_os = "linux")]
fn body_of(dialog: &gtk::Dialog, i18n: &DialogI18n, request: &SecretPromptRequest) -> gtk::Entry {
    use gtk::prelude::*;

    let content_area = dialog.content_area();
    content_area.set_spacing(12);
    content_area.set_margin_start(24);
    content_area.set_margin_end(24);
    content_area.set_margin_top(24);
    content_area.set_margin_bottom(24);
    content_area.pack_start(&heading_of(i18n), false, false, 0);

    if let Some(text) = &i18n.pin_warning {
        content_area.pack_start(&warning_label(text, request.pin_warning), false, false, 0);
    }

    let entry = masked_entry();
    content_area.pack_start(&entry, false, false, 0);

    if request.incorrect_secret {
        say_the_previous_attempt_was_wrong(&content_area, &entry, &i18n.incorrect_secret);
    }

    entry
}

#[cfg(target_os = "linux")]
fn emptied_into_a_result(
    entry: &gtk::Entry,
    response: gtk::ResponseType,
) -> Result<ProtectedSecret, SecretPromptError> {
    use gtk::prelude::*;

    let typed = entry.text();
    let secret = ProtectedSecret::new(typed.as_bytes());
    entry.set_text("");

    match response {
        gtk::ResponseType::Ok => Ok(secret),
        _ => Err(SecretPromptError::Cancelled),
    }
}

#[cfg(target_os = "linux")]
fn dismiss(dialog: gtk::Dialog) {
    use gtk::prelude::*;

    dialog.close();
    unsafe {
        dialog.destroy();
    }
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}

#[cfg(target_os = "linux")]
fn show_gtk_dialog(
    request: &SecretPromptRequest,
    parent: Option<gtk::ApplicationWindow>,
) -> Result<ProtectedSecret, SecretPromptError> {
    use gtk::prelude::*;

    if gtk::init().is_err() {
        return Err(SecretPromptError::Failed(
            "no se pudo inicializar GTK (sin entorno gráfico disponible)".to_string(),
        ));
    }

    let i18n = localize(request);
    let dialog = dialog_for(&i18n, parent);
    let entry = body_of(&dialog, &i18n, request);

    dialog.show_all();
    entry.grab_focus();

    let result = emptied_into_a_result(&entry, dialog.run());
    dismiss(dialog);
    result
}

#[cfg(target_os = "linux")]
impl SecretPrompter for GtkSecretPrompter {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        let context = glib::MainContext::default();
        if context.is_owner() {
            let parent = self.window_to_be_modal_over(request.origin_window);
            show_gtk_dialog(request, parent)
        } else {
            let (sender, receiver) = std::sync::mpsc::channel();
            let req = request.clone();
            let this = self.clone();
            context.invoke(move || {
                let parent = this.window_to_be_modal_over(req.origin_window);
                let res = show_gtk_dialog(&req, parent);
                let _ = sender.send(res);
            });
            receiver.recv().unwrap_or(Err(SecretPromptError::Cancelled))
        }
    }
}

/// El diálogo nativo del PIN en Windows, que aún no existe: toda petición falla.
#[cfg(windows)]
#[derive(Clone, Default)]
pub struct PendingWindowsPinDialog;

#[cfg(windows)]
impl PendingWindowsPinDialog {
    /// Crea el adaptador pendiente.
    pub fn new() -> Self {
        Self
    }

    /// No vincula nada: el diálogo de Windows no tiene ventana a la que atarse todavía.
    pub fn attach(&self, _app: tauri::AppHandle) {}
}

#[cfg(windows)]
impl SecretPrompter for PendingWindowsPinDialog {
    fn prompt_secret(
        &self,
        _request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        Err(SecretPromptError::Failed(
            "la solicitud del PIN aún no está disponible en Windows".to_owned(),
        ))
    }
}

/// El diálogo nativo del PIN en macOS, que aún no existe: toda petición falla.
#[cfg(target_os = "macos")]
#[derive(Clone, Default)]
pub struct PendingMacosPinDialog;

#[cfg(target_os = "macos")]
impl PendingMacosPinDialog {
    /// Crea el adaptador pendiente.
    pub fn new() -> Self {
        Self
    }

    /// No vincula nada: el diálogo de macOS no tiene ventana a la que atarse todavía.
    pub fn attach(&self, _app: tauri::AppHandle) {}
}

#[cfg(target_os = "macos")]
impl SecretPrompter for PendingMacosPinDialog {
    fn prompt_secret(
        &self,
        _request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        Err(SecretPromptError::Failed(
            "la solicitud del PIN aún no está disponible en macOS".to_owned(),
        ))
    }
}

/// El diálogo nativo del PIN de esta plataforma.
#[cfg(target_os = "linux")]
pub type NativePinDialog = GtkSecretPrompter;
/// El diálogo nativo del PIN de esta plataforma.
#[cfg(windows)]
pub type NativePinDialog = PendingWindowsPinDialog;
/// El diálogo nativo del PIN de esta plataforma.
#[cfg(target_os = "macos")]
pub type NativePinDialog = PendingMacosPinDialog;

/// Adaptador de pruebas que suministra un secreto fijo preconfigurado.
pub struct PreconfiguredSecretPrompter {
    secret: Mutex<Option<String>>,
    cancelled: bool,
}

impl PreconfiguredSecretPrompter {
    /// Crea un prompter que devuelve siempre el secreto suministrado.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: Mutex::new(Some(secret.into())),
            cancelled: false,
        }
    }

    /// Crea un prompter que simula la cancelación por el usuario.
    pub fn cancelling() -> Self {
        Self {
            secret: Mutex::new(None),
            cancelled: true,
        }
    }
}

impl SecretPrompter for PreconfiguredSecretPrompter {
    fn prompt_secret(
        &self,
        _request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        if self.cancelled {
            return Err(SecretPromptError::Cancelled);
        }
        let guard = self.secret.lock().unwrap();
        match guard.as_ref() {
            Some(s) => Ok(ProtectedSecret::from_str(s)),
            None => Err(SecretPromptError::Cancelled),
        }
    }
}

/// Adaptador de pruebas que registra las solicitudes recibidas y devuelve respuestas programadas.
pub struct MockSecretPrompter {
    responses: Mutex<Vec<Result<ProtectedSecret, SecretPromptError>>>,
    recorded_requests: Mutex<Vec<SecretPromptRequest>>,
}

impl MockSecretPrompter {
    /// Crea un nuevo prompter mock vacío.
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(Vec::new()),
            recorded_requests: Mutex::new(Vec::new()),
        }
    }

    /// Configura las respuestas sucesivas que devolverá el mock.
    pub fn with_responses(responses: Vec<Result<ProtectedSecret, SecretPromptError>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            recorded_requests: Mutex::new(Vec::new()),
        }
    }

    /// Configura una lista de secretos que devolverá el mock sucesivamente.
    pub fn with_secrets(secrets: &[&str]) -> Self {
        let responses = secrets
            .iter()
            .map(|s| Ok(ProtectedSecret::from_str(s)))
            .collect();
        Self::with_responses(responses)
    }

    /// Lista de solicitudes recibidas por el mock.
    pub fn recorded_requests(&self) -> Vec<SecretPromptRequest> {
        self.recorded_requests.lock().unwrap().clone()
    }
}

impl Default for MockSecretPrompter {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretPrompter for MockSecretPrompter {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        self.recorded_requests.lock().unwrap().push(request.clone());
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Err(SecretPromptError::Cancelled)
        } else {
            responses.remove(0)
        }
    }
}

#[cfg(test)]
mod tests;
