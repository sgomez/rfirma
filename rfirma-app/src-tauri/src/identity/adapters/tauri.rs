//! Las órdenes de identidad: certificados de los tokens y almacenes PKCS#12.

use tauri::State;

use crate::identity::IdentityRoot;

use super::views::CertificateView;
use crate::crossing::Failure;

/// Certificados de los tokens conectados.
#[tauri::command]
pub fn list_certificates(
    identity: State<'_, IdentityRoot>,
) -> Result<Vec<CertificateView>, Failure> {
    Ok(crate::identity::application::certificates::listed_rows(
        identity.token.as_ref(),
        &identity.all_stores(),
        identity.installed_certificates(),
        &identity.listed,
        identity.memory.as_ref(),
    )?
    .into_iter()
    .map(CertificateView::from)
    .collect())
}

/// Instala un fichero PKCS#12 en un almacén propio.
#[tauri::command(async)]
pub fn install_certificate(
    app_handle: tauri::AppHandle,
    identity: State<'_, IdentityRoot>,
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
        identity.token.as_ref(),
        identity.installed_certificates(),
        chosen,
        &password,
    )?;
    Ok(true)
}

/// Desinstala un certificado PKCS#12 previamente instalado.
#[tauri::command(async)]
pub fn remove_certificate(id: String, identity: State<'_, IdentityRoot>) -> Result<(), Failure> {
    Ok(
        crate::identity::application::certificates::remove_installed(
            identity.installed_certificates(),
            &id,
            &identity.listed,
        )?,
    )
}
