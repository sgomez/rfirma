//! Las órdenes del trámite de sede: desempaquetar, llamar a `app/errand/` y traducir.

use tauri::State;

use crate::identity::IdentityRoot;
use crate::site::SiteRoot;

use super::views::SiteErrandView;
use super::window::{self as site_window, SITE_WINDOW};
use crate::crossing::Failure;
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
pub fn site_decline(site: State<'_, SiteRoot>) {
    crate::site::application::errand::decline(&site.errand);
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
    Ok(site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::finish(desk, live)
    })?)
}

/// Abre el diálogo para instalar un certificado desde la ventana de sede.
#[tauri::command(async)]
pub fn site_install_certificate(
    app_handle: tauri::AppHandle,
    identity: State<'_, IdentityRoot>,
    password: String,
) -> Result<bool, Failure> {
    install_certificate(app_handle, identity, password)
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
pub fn install_local_ca(app_handle: tauri::AppHandle, site: State<'_, SiteRoot>) {
    crate::site::application::startup::repair_the_local_ca(
        &site.trust,
        &site.held_channel,
        &site.errand,
    );
    site_window::publish_the_moment(&app_handle);
}

/// Consulta el momento actual del trámite de sede.
#[tauri::command]
pub fn read_site_errand(site: State<'_, SiteRoot>) -> Option<SiteErrandView> {
    site.errand.moment().as_ref().map(SiteErrandView::from)
}
