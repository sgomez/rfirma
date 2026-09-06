//! Las órdenes del trámite de sede: desempaquetar, llamar a `app/errand/` y traducir.

use tauri::State;

use crate::Environment;

use super::views::SiteErrandView;
use super::window::{self as site_window, SITE_WINDOW};
use crate::commands::Failure;
use crate::identity::adapters::tauri::install_certificate;
use crate::identity::adapters::views::SecretView;

/// Cierra la ventana del trámite de sede.
#[tauri::command(async)]
pub fn close_site_window(app: tauri::AppHandle) {
    use tauri::Manager as _;

    if let Some(window) = app.get_webview_window(SITE_WINDOW) {
        let _ = window.close();
    }
}

/// Identifica a la persona ante la sede con el certificado elegido.
#[tauri::command(async)]
pub fn site_identify(certificate: String, app_handle: tauri::AppHandle) -> Result<(), Failure> {
    site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::consent(desk, &certificate, live)
    })?;
    Ok(())
}

/// Cancela el trámite ante la sede.
#[tauri::command(async)]
pub fn site_decline(live: State<'_, crate::site::application::errand::LiveErrand>) {
    crate::site::application::errand::decline(&live);
}

/// Inicia la firma del trámite de sede con el certificado elegido (ADR-0001).
#[tauri::command(async)]
pub fn site_begin_signing(
    certificate: String,
    app_handle: tauri::AppHandle,
) -> Result<SecretView, Failure> {
    let consented = site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::consent(desk, &certificate, live)
    })?;
    match consented {
        crate::site::application::errand::Consented::SigningWith(secret) => {
            Ok(SecretView::from(secret))
        }
        crate::site::application::errand::Consented::IdentityHandedOver => Err(Failure::new(
            "siteErrandNotLive",
            "lo que habia pendiente era una identificacion, y ya se ha entregado",
        )),
    }
}

/// Postfirma del trámite de sede y entrega del resultado a la sede.
#[tauri::command(async)]
pub fn site_finish_signing(app_handle: tauri::AppHandle) -> Result<(), Failure> {
    Ok(site_window::with_the_desk(
        &app_handle,
        crate::site::application::errand::finish,
    )?)
}

/// Abre el diálogo para instalar un certificado desde la ventana de sede.
#[tauri::command(async)]
pub fn site_install_certificate(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    password: String,
) -> Result<bool, Failure> {
    install_certificate(app_handle, environment, password)
}

/// Vuelve a consultar los certificados disponibles en el trámite de sede.
#[tauri::command(async)]
pub fn site_look_again(app_handle: tauri::AppHandle) {
    let looked = site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::look_again(desk, live)
    });
    site_window::publish_what_moved(&app_handle, looked);
}

/// Instala la CA local en los almacenes NSS del usuario (ADR-0005).
#[tauri::command(async)]
pub fn install_local_ca(
    app_handle: tauri::AppHandle,
    trust: State<'_, crate::site::application::startup::LocalCaTrust>,
    held: State<'_, crate::site::application::startup::HeldChannel>,
    live: State<'_, crate::site::application::errand::LiveErrand>,
) {
    crate::site::application::startup::repair_the_local_ca(&trust, &held, &live);
    site_window::publish_the_moment(&app_handle);
}

/// Consulta el momento actual del trámite de sede.
#[tauri::command]
pub fn read_site_errand(
    live: State<'_, crate::site::application::errand::LiveErrand>,
) -> Option<SiteErrandView> {
    live.moment().as_ref().map(SiteErrandView::from)
}
