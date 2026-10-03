//! `SiteWindowPicker`, el elector gráfico de `-certgui`: levanta Tauri en el proceso de terminal solo para la ventana de sede con el origen «orden de terminal», y devuelve lo elegido en ella; no firma.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::crossing::Failure;
use crate::desktop::adapters::paths::Platform;
use crate::desktop::adapters::views::{TerminalChoiceView, TerminalStageView};
use crate::desktop::ports::{GraphicalPicker, WindowChoice, WindowOffer};
use crate::identity::adapters::views::CertificateView;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::error::{Situation, TokenError};
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::identity::domain::secret::StoreSecret;
use crate::identity::IdentityRoot;
use crate::lock;
use crate::signing::ports::Signer as _;

/// La etiqueta de la ventana, la de sede, para que la cubran sus permisos.
pub const TERMINAL_WINDOW: &str = "site";

const DIALOG_SIZE: (f64, f64) = (520.0, 420.0);

/// La ventana de sede de `-certgui`, sobre la identidad de este proceso y el contexto de Tauri del crate.
pub struct SiteWindowPicker<'a> {
    identity: Option<&'a IdentityRoot>,
    context: Mutex<Option<tauri::Context<tauri::Wry>>>,
}

impl<'a> SiteWindowPicker<'a> {
    /// El elector de esa identidad, o uno que no abre nada si no se sabe dónde vive la memoria.
    pub fn over(identity: Option<&'a IdentityRoot>, context: tauri::Context<tauri::Wry>) -> Self {
        Self {
            identity,
            context: Mutex::new(Some(context)),
        }
    }
}

impl GraphicalPicker for SiteWindowPicker<'_> {
    fn has_a_display(&self) -> bool {
        has_a_display(Platform::CURRENT, |name| std::env::var_os(name))
    }

    fn chosen(&self, document: &Path, offer: WindowOffer<'_>) -> Result<WindowChoice, String> {
        let identity = self
            .identity
            .ok_or_else(|| "no se sabe cuál es la carpeta personal".to_owned())?;
        let context = lock(&self.context)
            .take()
            .ok_or_else(|| "la ventana de sede solo se abre una vez por orden".to_owned())?;
        let choice = Arc::new(TerminalChoice::of(identity, document, offer));
        let app = tauri::Builder::default()
            .manage(Arc::clone(&choice))
            .invoke_handler(tauri::generate_handler![
                super::tauri_certgui::read_terminal_choice,
                super::tauri_certgui::terminal_choose_certificate,
                super::tauri_certgui::terminal_hand_the_secret,
                super::tauri_certgui::terminal_cancel,
            ])
            .setup({
                let choice = Arc::clone(&choice);
                move |app| {
                    if let Err(error) = open_the_window(app.handle()) {
                        choice.could_not_open(error.to_string());
                        app.handle().exit(0);
                    }
                    Ok(())
                }
            })
            .build(context)
            .map_err(|error| error.to_string())?;
        app.run_return(|_, _| {});
        choice.taken()
    }
}

/// Si el entorno dice que hay pantalla: en Linux, un `WAYLAND_DISPLAY` o un `DISPLAY` no vacíos.
pub fn has_a_display(
    platform: Platform,
    variable: impl Fn(&str) -> Option<std::ffi::OsString>,
) -> bool {
    if platform != Platform::Linux {
        return true;
    }
    ["WAYLAND_DISPLAY", "DISPLAY"]
        .into_iter()
        .any(|name| variable(name).is_some_and(|value| !value.is_empty()))
}

fn open_the_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    WebviewWindowBuilder::new(
        app,
        TERMINAL_WINDOW,
        WebviewUrl::App("sede.html?origin=terminal".into()),
    )
    .title("rFirma")
    .inner_size(DIALOG_SIZE.0, DIALOG_SIZE.1)
    .resizable(false)
    .center()
    .focused(true)
    .build()
    .map(|_| ())
}

/// El asa de una fila de la ventana, y su certificado con el secreto que pide su almacén, o por qué no se puede elegir.
pub type Choosable = (String, Result<(TokenCertificate, StoreSecret), Failure>);

/// Lo que sabe la ventana de `-certgui` mientras está abierta, y lo que la persona ha elegido en ella.
pub struct TerminalChoice {
    view: TerminalChoiceView,
    choosable: Vec<Choosable>,
    pending: Mutex<Option<TokenCertificate>>,
    choice: Mutex<Option<WindowChoice>>,
    unopened: Mutex<Option<String>>,
}

impl TerminalChoice {
    /// La elección que se ofrece para ese documento, con las asas que acuña la identidad.
    pub fn of(identity: &IdentityRoot, document: &Path, offer: WindowOffer<'_>) -> Self {
        let (stage, choosable) = match offer {
            WindowOffer::Nothing(nothing) => (TerminalStageView::from(nothing), Vec::new()),
            WindowOffer::Certificates(certificates) => {
                let rows = identity.rows_of(certificates.to_vec());
                let choosable = rows
                    .iter()
                    .map(|row| {
                        let usable = identity
                            .usable(certificates, &row.id)
                            .map(|certificate| {
                                let secret = identity
                                    .signer()
                                    .secret_of(certificate.reference())
                                    .unwrap_or(StoreSecret::TypedOnScreen);
                                (certificate.clone(), secret)
                            })
                            .map_err(Failure::from);
                        (row.id.clone(), usable)
                    })
                    .collect();
                let certificates = rows.into_iter().map(CertificateView::from).collect();
                (TerminalStageView::AskingToSign { certificates }, choosable)
            }
        };
        Self::over(TerminalChoiceView::of(document, stage), choosable)
    }

    /// La elección que enseña esa vista y deja elegir esas filas.
    pub fn over(view: TerminalChoiceView, choosable: Vec<Choosable>) -> Self {
        Self {
            view,
            choosable,
            pending: Mutex::new(None),
            choice: Mutex::new(None),
            unopened: Mutex::new(None),
        }
    }

    /// Lo que la ventana enseña.
    pub fn view(&self) -> TerminalChoiceView {
        self.view.clone()
    }

    /// Elige el certificado del asa y dice qué secreto pide; si no pide ninguno, la elección queda hecha.
    pub fn choose(&self, handle: &str) -> Result<StoreSecret, Failure> {
        let (certificate, secret) = self
            .choosable
            .iter()
            .find(|(offered, _)| offered == handle)
            .ok_or_else(|| {
                Failure::from(TokenError::new(
                    Situation::CertificateNotFound,
                    format!("la ventana no ofrecía el certificado «{handle}»"),
                ))
            })?
            .1
            .clone()?;
        if secret == StoreSecret::TypedOnScreen {
            *lock(&self.pending) = Some(certificate);
        } else {
            self.decide(WindowChoice::Chosen {
                certificate: Box::new(certificate),
                secret: None,
            });
        }
        Ok(secret)
    }

    /// Completa con el secreto tecleado la elección del certificado que lo pedía.
    pub fn hand_the_secret(&self, secret: ProtectedSecret) -> Result<(), Failure> {
        let certificate = lock(&self.pending).take().ok_or_else(|| {
            Failure::from(TokenError::new(
                Situation::CertificateNotFound,
                "no hay ningún certificado elegido que espere el PIN",
            ))
        })?;
        self.decide(WindowChoice::Chosen {
            certificate: Box::new(certificate),
            secret: Some(secret),
        });
        Ok(())
    }

    /// La persona cancela, por el botón o cerrando la ventana.
    pub fn cancel(&self) {
        self.decide(WindowChoice::Cancelled);
    }

    /// Si la elección ya está hecha y la ventana puede cerrarse.
    pub fn is_decided(&self) -> bool {
        lock(&self.choice).is_some()
    }

    fn decide(&self, choice: WindowChoice) {
        lock(&self.choice).get_or_insert(choice);
    }

    fn could_not_open(&self, reason: String) {
        *lock(&self.unopened) = Some(reason);
    }

    /// Lo elegido al cerrarse la ventana: sin elección es que se canceló.
    pub fn taken(&self) -> Result<WindowChoice, String> {
        if let Some(reason) = lock(&self.unopened).take() {
            return Err(reason);
        }
        Ok(lock(&self.choice).take().unwrap_or(WindowChoice::Cancelled))
    }
}

#[cfg(test)]
mod tests;
