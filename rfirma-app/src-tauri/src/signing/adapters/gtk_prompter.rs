//! Adaptadores del puerto `SecretPrompter`: diálogo nativo GTK3 y adaptadores de pruebas (ADR-0001, ADR-0014).

use std::sync::Mutex;

use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;
use crate::signing::ports::{SecretName, SecretPromptError, SecretPromptRequest, SecretPrompter};

/// Estructura interna con los textos localizados para el diálogo modal del secreto.
#[derive(Debug, PartialEq, Eq)]
pub struct DialogI18n {
    pub title: &'static str,
    pub holder_name: Option<String>,
    pub id_number: Option<String>,
    pub incorrect_secret: &'static str,
    pub accept: &'static str,
    pub cancel: &'static str,
}

fn title_text(lang: Language, secret: SecretName) -> &'static str {
    match (lang, secret) {
        (Language::Spanish, SecretName::Pin) => "Introduce el PIN",
        (Language::Spanish, SecretName::Password) => "Introduce la contraseña",

        (Language::Catalan, SecretName::Pin) => "Introdueix el PIN",
        (Language::Catalan, SecretName::Password) => "Introdueix la contrasenya",

        (Language::Basque, SecretName::Pin) => "Sartu PINa",
        (Language::Basque, SecretName::Password) => "Sartu pasahitza",

        (Language::Galician, SecretName::Pin) => "Introduce o PIN",
        (Language::Galician, SecretName::Password) => "Introduce o contrasinal",

        (Language::English, SecretName::Pin) => "Enter PIN",
        (Language::English, SecretName::Password) => "Enter password",
    }
}

fn button_texts(lang: Language) -> (&'static str, &'static str) {
    match lang {
        Language::Spanish => ("Aceptar", "Cancelar"),
        Language::Catalan => ("Acceptar", "Cancel·lar"),
        Language::Basque => ("Onartu", "Utzi"),
        Language::Galician => ("Aceptar", "Cancelar"),
        Language::English => ("OK", "Cancel"),
    }
}

fn incorrect_secret_text(lang: Language, secret: SecretName) -> &'static str {
    match (lang, secret) {
        (Language::Spanish, SecretName::Pin) => "PIN incorrecto. Vuelve a intentarlo.",
        (Language::Spanish, SecretName::Password) => "Contraseña incorrecta. Vuelve a intentarlo.",

        (Language::Catalan, SecretName::Pin) => "PIN incorrecte. Torna-ho a provar.",
        (Language::Catalan, SecretName::Password) => "Contrasenya incorrecta. Torna-ho a provar.",

        (Language::Basque, SecretName::Pin) => "PIN okerra. Saiatu berriro.",
        (Language::Basque, SecretName::Password) => "Pasahitza okerra. Saiatu berriro.",

        (Language::Galician, SecretName::Pin) => "PIN incorrecto. Tenta de novo.",
        (Language::Galician, SecretName::Password) => "Contrasinal incorrecto. Tenta de novo.",

        (Language::English, SecretName::Pin) => "Incorrect PIN. Try again.",
        (Language::English, SecretName::Password) => "Incorrect password. Try again.",
    }
}

/// Traduce los textos del diálogo a uno de los 5 idiomas oficiales de rFirma (ADR-0009).
pub fn localize(request: &SecretPromptRequest) -> DialogI18n {
    let (accept, cancel) = button_texts(request.language);

    DialogI18n {
        title: title_text(request.language, request.secret),
        holder_name: request.holder.as_ref().map(|holder| holder.name.clone()),
        id_number: request
            .holder
            .as_ref()
            .map(|holder| holder.id_number.clone())
            .filter(|id_number| !id_number.is_empty()),
        incorrect_secret: incorrect_secret_text(request.language, request.secret),
        accept,
        cancel,
    }
}

/// Adaptador de producción que presenta un diálogo modal nativo GTK3 para la solicitud de PIN.
#[derive(Default, Clone, Copy)]
pub struct GtkSecretPrompter;

fn window_to_be_modal_over() -> Option<gtk::Window> {
    use gtk::prelude::*;

    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|toplevel| toplevel.downcast::<gtk::Window>().ok())
        .find(|window| window.is_visible() && window.is_mapped())
}

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
        glib::markup_escape_text(i18n.holder_name.as_deref().unwrap_or(i18n.title))
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

fn dialog_for(i18n: &DialogI18n) -> gtk::Dialog {
    use gtk::prelude::*;

    let dialog = gtk::Dialog::builder()
        .title(i18n.title)
        .modal(true)
        .resizable(false)
        .icon_name("dialog-password")
        .build();
    dialog.set_default_size(400, -1);

    match window_to_be_modal_over() {
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

fn failure_label(text: &str) -> gtk::Label {
    use gtk::prelude::*;

    let failure = gtk::Label::new(None);
    failure.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(text)));
    failure.set_halign(gtk::Align::Start);
    failure.set_xalign(0.0);
    failure.set_line_wrap(true);
    failure
}

fn say_the_previous_attempt_was_wrong(content_area: &gtk::Box, entry: &gtk::Entry, text: &str) {
    use gtk::prelude::*;

    entry.style_context().add_class("error");
    content_area.pack_start(&failure_label(text), false, false, 0);
}

fn body_of(dialog: &gtk::Dialog, i18n: &DialogI18n, incorrect_secret: bool) -> gtk::Entry {
    use gtk::prelude::*;

    let content_area = dialog.content_area();
    content_area.set_spacing(12);
    content_area.set_margin_start(24);
    content_area.set_margin_end(24);
    content_area.set_margin_top(24);
    content_area.set_margin_bottom(24);
    content_area.pack_start(&heading_of(i18n), false, false, 0);

    let entry = masked_entry();
    content_area.pack_start(&entry, false, false, 0);

    if incorrect_secret {
        say_the_previous_attempt_was_wrong(&content_area, &entry, i18n.incorrect_secret);
    }

    entry
}

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

fn show_gtk_dialog(request: &SecretPromptRequest) -> Result<ProtectedSecret, SecretPromptError> {
    use gtk::prelude::*;

    if gtk::init().is_err() {
        return Err(SecretPromptError::Failed(
            "no se pudo inicializar GTK (sin entorno gráfico disponible)".to_string(),
        ));
    }

    let i18n = localize(request);
    let dialog = dialog_for(&i18n);
    let entry = body_of(&dialog, &i18n, request.incorrect_secret);

    dialog.show_all();
    entry.grab_focus();

    let result = emptied_into_a_result(&entry, dialog.run());
    dismiss(dialog);
    result
}

impl SecretPrompter for GtkSecretPrompter {
    fn prompt_secret(
        &self,
        request: &SecretPromptRequest,
    ) -> Result<ProtectedSecret, SecretPromptError> {
        let context = glib::MainContext::default();
        if context.is_owner() {
            show_gtk_dialog(request)
        } else {
            let (sender, receiver) = std::sync::mpsc::channel();
            let req = request.clone();
            context.invoke(move || {
                let res = show_gtk_dialog(&req);
                let _ = sender.send(res);
            });
            receiver.recv().unwrap_or(Err(SecretPromptError::Cancelled))
        }
    }
}

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
