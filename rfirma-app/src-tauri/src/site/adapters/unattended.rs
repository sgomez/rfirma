//! El consentimiento sin nadie delante de la compilación de conformidad, encendido por `RFIRMA_CONFORMANCE_AUTOCONSENT=1`; no existe en un binario publicado.

use tauri::Manager as _;

use crate::site::application::errand::unattended;
use crate::site::application::errand::ErrandStep;

use super::window::{with_the_desk, SITE_WINDOW};

const SWITCH: &str = "RFIRMA_CONFORMANCE_AUTOCONSENT";

/// Consiente por el camino del clic si el interruptor está encendido y el paso no deja nada que decidir.
pub fn consent_if_nothing_to_decide(app: &tauri::AppHandle, step: Option<&ErrandStep>) {
    if !unattended::switched_on(std::env::var_os(SWITCH).as_deref()) {
        return;
    }
    let Some(unattended) = step.and_then(|step| {
        with_the_desk(app, |desk, _| unattended::nothing_to_decide_on(desk, step))
    }) else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        eprintln!("rfirma: {SWITCH}: se consiente sin esperar a la persona");
        with_the_desk(&app, |desk, live| {
            unattended::consent_unattended(desk, live, &unattended, || {
                crate::signing::adapters::tauri::signed_with_the_secret(desk, live, "").is_ok()
            });
        });
        if let Some(window) = app.get_webview_window(SITE_WINDOW) {
            let _ = window.close();
        }
    });
}
