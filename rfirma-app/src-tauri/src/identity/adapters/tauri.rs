//! Las órdenes de identidad: listar los certificados e instalar o quitar un `.p12`.

use tauri::State;

use crate::identity::IdentityRoot;
use crate::signing::SigningRoot;

use super::views::CertificateView;
use crate::crossing::Failure;
use crate::identity::application::certificates::PasswordPrompt;
use crate::identity::ports::OriginWindow;

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
        &identity.installed_copies,
        identity.memory.as_ref(),
        &identity.last_listing,
    )?
    .into_iter()
    .map(CertificateView::from)
    .collect())
}

/// Abre el selector de fichero y, con el elegido, pide su contraseña e instala el `.p12`.
#[tauri::command(async)]
pub fn install_certificate(
    app_handle: tauri::AppHandle,
    identity: State<'_, IdentityRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<bool, Failure> {
    install_certificate_over(app_handle, &identity, &signing, OriginWindow::Main)
}

/// Instala un `.p12` elegido por la persona, con el diálogo de su contraseña modal sobre `origin_window`.
pub fn install_certificate_over(
    app_handle: tauri::AppHandle,
    identity: &IdentityRoot,
    signing: &SigningRoot,
    origin_window: OriginWindow,
) -> Result<bool, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Certificado", &["p12", "pfx"]);
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(false);
    };
    let (file_name, pkcs12) = read_the_file(chosen)?;
    let keyring = (identity.keyring)()?;

    super::failures::installed_unless_cancelled(
        crate::identity::application::certificates::install_pkcs12_asking_its_password(
            identity.token.as_ref(),
            identity.folder.as_ref(),
            keyring.as_ref(),
            identity.installed_certificates(),
            &pkcs12,
            &file_name,
            PasswordPrompt {
                prompter: identity.prompter.as_ref(),
                language: signing.configuration().language,
                origin_window,
            },
        ),
    )
}

/// Desinstala un certificado PKCS#12 previamente instalado.
#[tauri::command(async)]
pub fn remove_certificate(id: String, identity: State<'_, IdentityRoot>) -> Result<(), Failure> {
    let keyring = (identity.keyring)()?;
    Ok(
        crate::identity::application::certificates::remove_installed(
            identity.token.as_ref(),
            keyring.as_ref(),
            identity.memory.as_ref(),
            identity.installed_certificates(),
            &id,
            &identity.listed,
            &identity.installed_copies,
        )?,
    )
}

/// Vacía el Almacén de rFirma entero: solo se llama tras confirmarlo la persona, cuando el llavero perdió su PIN (ADR-0034).
#[tauri::command(async)]
pub fn empty_installed_store(identity: State<'_, IdentityRoot>) -> Result<(), Failure> {
    Ok(crate::identity::application::certificates::empty_the_store(
        identity.folder.as_ref(),
        identity.installed_certificates(),
    )?)
}

/// El nombre del fichero elegido y sus bytes.
fn read_the_file(chosen: tauri_plugin_dialog::FilePath) -> Result<(String, Vec<u8>), Failure> {
    let unreadable = |detail: String| {
        Failure::from(crate::identity::domain::error::TokenError::new(
            crate::identity::domain::error::Situation::Pkcs12Unreadable,
            detail,
        ))
    };
    let source = chosen
        .into_path()
        .map_err(|error| unreadable(error.to_string()))?;
    let file_name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::read(&source).map_err(|error| unreadable(error.to_string()))?;
    Ok((file_name, bytes))
}
