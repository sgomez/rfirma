//! Las órdenes de identidad: certificados de los tokens y almacenes PKCS#12.

use tauri::State;

use crate::Environment;

use super::views::CertificateView;
use crate::commands::Failure;

/// Certificados de los tokens conectados.
#[tauri::command]
pub fn list_certificates(
    environment: State<'_, Environment>,
) -> Result<Vec<CertificateView>, Failure> {
    crate::identity::application::certificates::listed_rows(
        &environment.all_stores(),
        &environment.installed_certificates,
        &environment.listed,
        &environment.memory,
    )
}

/// Instala un fichero PKCS#12 en un almacén propio.
#[tauri::command(async)]
pub fn install_certificate(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    password: String,
) -> Result<bool, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Certificado", &["p12", "pfx"]);
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(false);
    };

    crate::identity::application::certificates::install_pkcs12(
        &environment.installed_certificates,
        chosen,
        &password,
    )
    .map(|()| true)
}

/// Desinstala un certificado PKCS#12 previamente instalado.
#[tauri::command(async)]
pub fn remove_certificate(id: String, environment: State<'_, Environment>) -> Result<(), Failure> {
    crate::identity::application::certificates::remove_installed(
        &environment.installed_certificates,
        &id,
        &environment.listed,
    )
}
