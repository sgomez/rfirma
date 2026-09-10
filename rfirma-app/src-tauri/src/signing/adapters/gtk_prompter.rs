//! Adaptadores del puerto `SecretPrompter`: diálogo nativo GTK3 y adaptadores de pruebas (ADR-0001, ADR-0014).

use std::sync::Mutex;

use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::domain::Language;
use crate::signing::ports::{SecretPromptError, SecretPromptRequest, SecretPrompter};

/// Estructura interna con los textos localizados para el diálogo modal de PIN.
#[derive(Debug, PartialEq, Eq)]
pub struct DialogI18n {
    pub title: &'static str,
    pub prompt: String,
    pub incorrect_pin: String,
    pub accept: &'static str,
    pub cancel: &'static str,
}

fn button_texts(lang: Language) -> (&'static str, &'static str, &'static str) {
    match lang {
        Language::Spanish => ("Introduce el PIN", "Aceptar", "Cancelar"),
        Language::Catalan => ("Introdueix el PIN", "Acceptar", "Cancel·lar"),
        Language::Basque => ("Sartu PINa", "Onartu", "Utzi"),
        Language::Galician => ("Introduce o PIN", "Aceptar", "Cancelar"),
        Language::English => ("Enter PIN", "OK", "Cancel"),
    }
}

fn prompt_text(lang: Language, token_label: &str, subject: Option<&str>) -> String {
    match (lang, subject) {
        (Language::Spanish, Some(subj)) => {
            format!("Introduce el PIN para usar el certificado de {subj}:")
        }
        (Language::Spanish, None) => format!("Introduce el PIN del almacén {token_label}:"),

        (Language::Catalan, Some(subj)) => {
            format!("Introdueix el PIN per utilitzar el certificat de {subj}:")
        }
        (Language::Catalan, None) => format!("Introdueix el PIN del magatzem {token_label}:"),

        (Language::Basque, Some(subj)) => {
            format!("Sartu PINa {subj}-(r)en ziurtagiria erabiltzeko:")
        }
        (Language::Basque, None) => format!("Sartu {token_label} biltegiko PINa:"),

        (Language::Galician, Some(subj)) => {
            format!("Introduce o PIN para usar o certificado de {subj}:")
        }
        (Language::Galician, None) => format!("Introduce o PIN do almacén {token_label}:"),

        (Language::English, Some(subj)) => {
            format!("Enter the PIN to use the certificate for {subj}:")
        }
        (Language::English, None) => format!("Enter the PIN for {token_label}:"),
    }
}

fn incorrect_pin_text(lang: Language, attempts_left: Option<u32>) -> String {
    match (lang, attempts_left) {
        (Language::Spanish, Some(left)) => {
            format!("PIN incorrecto. Te quedan {left} intentos.")
        }
        (Language::Spanish, None) => "PIN incorrecto. Vuelve a intentarlo.".to_string(),

        (Language::Catalan, Some(left)) => {
            format!("PIN incorrecte. Et queden {left} intents.")
        }
        (Language::Catalan, None) => "PIN incorrecte. Torna-ho a provar.".to_string(),

        (Language::Basque, Some(left)) => {
            format!("PIN okerra. {left} saiakera geratzen zaizkizu.")
        }
        (Language::Basque, None) => "PIN okerra. Saiatu berriro.".to_string(),

        (Language::Galician, Some(left)) => {
            format!("PIN incorrecto. Quedanche {left} intentos.")
        }
        (Language::Galician, None) => "PIN incorrecto. Tenta de novo.".to_string(),

        (Language::English, Some(left)) => {
            format!("Incorrect PIN. {left} attempts remaining.")
        }
        (Language::English, None) => "Incorrect PIN. Try again.".to_string(),
    }
}

/// Traduce los textos del diálogo a uno de los 5 idiomas oficiales de rFirma (ADR-0009).
pub fn localize(request: &SecretPromptRequest) -> DialogI18n {
    let (title, accept, cancel) = button_texts(request.language);
    let prompt = prompt_text(
        request.language,
        &request.token_label,
        request.subject.as_deref(),
    );
    let incorrect_pin = incorrect_pin_text(request.language, request.attempts_left);

    DialogI18n {
        title,
        prompt,
        incorrect_pin,
        accept,
        cancel,
    }
}

/// Adaptador de producción que presenta un diálogo modal nativo GTK3 para la solicitud de PIN.
#[derive(Default, Clone, Copy)]
pub struct GtkSecretPrompter;

fn show_gtk_dialog(request: &SecretPromptRequest) -> Result<ProtectedSecret, SecretPromptError> {
    use gtk::prelude::*;

    if gtk::init().is_err() {
        return Err(SecretPromptError::Failed(
            "no se pudo inicializar GTK (sin entorno gráfico disponible)".to_string(),
        ));
    }

    let i18n = localize(request);

    let dialog = gtk::Dialog::new();
    dialog.set_title(i18n.title);
    dialog.set_modal(true);
    dialog.set_position(gtk::WindowPosition::Center);
    dialog.set_default_size(360, -1);
    dialog.set_resizable(false);

    dialog.add_button(i18n.cancel, gtk::ResponseType::Cancel);
    dialog.add_button(i18n.accept, gtk::ResponseType::Ok);
    dialog.set_default_response(gtk::ResponseType::Ok);

    let content_area = dialog.content_area();
    content_area.set_spacing(10);
    content_area.set_margin_start(16);
    content_area.set_margin_end(16);
    content_area.set_margin_top(16);
    content_area.set_margin_bottom(16);

    let prompt_label = gtk::Label::new(Some(&i18n.prompt));
    prompt_label.set_halign(gtk::Align::Start);
    prompt_label.set_line_wrap(true);
    content_area.pack_start(&prompt_label, false, false, 0);

    if request.incorrect_pin {
        let error_label = gtk::Label::new(None);
        let markup = format!(
            "<span foreground=\'#e01b24\'><b>{}</b></span>",
            glib::markup_escape_text(&i18n.incorrect_pin)
        );
        error_label.set_markup(&markup);
        error_label.set_halign(gtk::Align::Start);
        error_label.set_line_wrap(true);
        content_area.pack_start(&error_label, false, false, 0);
    }

    let entry = gtk::Entry::new();
    entry.set_visibility(false);
    entry.set_invisible_char(Some('•'));
    entry.set_activates_default(true);
    entry.set_width_chars(24);
    content_area.pack_start(&entry, false, false, 4);

    dialog.show_all();

    let response = dialog.run();

    let result = if response == gtk::ResponseType::Ok {
        let text = entry.text();
        let secret = ProtectedSecret::new(text.as_bytes());
        entry.set_text("");
        Ok(secret)
    } else {
        entry.set_text("");
        Err(SecretPromptError::Cancelled)
    };

    dialog.close();
    unsafe {
        dialog.destroy();
    }

    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }

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
