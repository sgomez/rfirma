//! Las órdenes de la ventana de sede de `-certgui`, que solo registra el proceso de terminal; ninguna firma.

use std::sync::Arc;

use tauri::{Manager as _, State};

use super::site_window_picker::{TerminalChoice, TERMINAL_WINDOW};
use super::views::TerminalChoiceView;
use crate::crossing::Failure;
use crate::identity::adapters::views::SecretView;
use crate::identity::domain::protected_secret::ProtectedSecret;

/// El documento de la orden y lo que la ventana enseña.
#[tauri::command(async)]
pub fn read_terminal_choice(choice: State<'_, Arc<TerminalChoice>>) -> TerminalChoiceView {
    choice.view()
}

/// Elige el certificado del asa y dice qué secreto pide; si no pide ninguno, cierra la ventana.
#[tauri::command(async)]
pub fn terminal_choose_certificate(
    certificate: String,
    choice: State<'_, Arc<TerminalChoice>>,
    app_handle: tauri::AppHandle,
) -> Result<SecretView, Failure> {
    let secret = choice.choose(&certificate)?;
    if choice.is_decided() {
        close_the_window(&app_handle);
    }
    Ok(secret.into())
}

/// Devuelve a la orden el PIN del certificado elegido y cierra la ventana.
#[tauri::command(async)]
pub fn terminal_hand_the_secret(
    pin: String,
    choice: State<'_, Arc<TerminalChoice>>,
    app_handle: tauri::AppHandle,
) -> Result<(), Failure> {
    choice.hand_the_secret(ProtectedSecret::new(pin))?;
    close_the_window(&app_handle);
    Ok(())
}

/// Cancela la elección y cierra la ventana.
#[tauri::command(async)]
pub fn terminal_cancel(choice: State<'_, Arc<TerminalChoice>>, app_handle: tauri::AppHandle) {
    choice.cancel();
    close_the_window(&app_handle);
}

fn close_the_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(TERMINAL_WINDOW) {
        let _ = window.close();
    }
}
