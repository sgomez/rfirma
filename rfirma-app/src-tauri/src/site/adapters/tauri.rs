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

/// Sigue con la firma tras confirmar lo que el validador del original señaló (`checkSignatures`).
#[tauri::command(async)]
pub fn site_confirm_signatures(app_handle: tauri::AppHandle) -> Result<(), Failure> {
    let moved = site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::confirm(desk, live)
    })?;
    site_window::publish_what_moved(&app_handle, Some(moved));
    Ok(())
}

/// Postfirma del trámite de sede y entrega del resultado a la sede, o el paso al guardado
/// del portal si la firma venía de `signandsave`.
#[tauri::command(async)]
pub fn site_finish_signing(app_handle: tauri::AppHandle) -> Result<(), Failure> {
    let moved = site_window::with_the_desk(&app_handle, |desk, live| {
        crate::site::application::errand::finish(desk, live)
    })?;
    site_window::publish_what_moved(&app_handle, moved);
    Ok(())
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

/// Añade el filtro de extensiones al diálogo del portal, si la sede declaró alguna.
fn with_extensions<R: tauri::Runtime>(
    mut dialog: tauri_plugin_dialog::FileDialogBuilder<R>,
    extensions: &[String],
    description: Option<&str>,
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    if extensions.is_empty() {
        return dialog;
    }
    let list: Vec<&str> = extensions.iter().map(String::as_str).collect();
    dialog = dialog.add_filter(description.unwrap_or(""), &list);
    dialog
}

/// El nombre base y la ruta de cada fichero que la persona eligió.
fn named_paths(
    chosen: Vec<tauri_plugin_dialog::FilePath>,
) -> Result<Vec<(String, std::path::PathBuf)>, Failure> {
    chosen
        .into_iter()
        .map(|file_path| {
            let path = file_path
                .into_path()
                .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            Ok((name, path))
        })
        .collect()
}

/// Un rechazo porque no hay ningún guardado o ninguna carga pendiente que atender.
fn nothing_pending(what: &str) -> Failure {
    Failure::new(
        "siteErrandNotLive",
        format!("no hay {what} pendiente que atender"),
    )
}

/// El diálogo de guardado del portal con las pistas que declaró la sede.
fn save_dialog<R: tauri::Runtime>(
    mut dialog: tauri_plugin_dialog::FileDialogBuilder<R>,
    consent: &crate::site::application::errand::SavingConsent,
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    if let Some(name) = consent.filename.as_deref() {
        dialog = dialog.set_file_name(name);
    }
    if let Some(title) = consent.title.as_deref() {
        dialog = dialog.set_title(title);
    }
    if let Some(folder) = consent.starting_folder.as_deref() {
        dialog = dialog.set_directory(folder);
    }
    with_extensions(dialog, &consent.extensions, consent.description.as_deref())
}

/// El diálogo de guardado se cerró sin elegir ningún destino.
fn save_cancelled() -> Failure {
    Failure::new(
        "saveCancelled",
        "el dialogo de guardado se cerro sin elegir nada",
    )
}

/// El diálogo de carga se cerró sin elegir ningún fichero.
fn load_cancelled() -> Failure {
    Failure::new(
        "loadCancelled",
        "el dialogo de carga se cerro sin elegir nada",
    )
}

/// Traduce el desenlace de un guardado a lo que espera la ventana: `true` si hay que enseñar el
/// desenlace de guardado, `false` si el guardado era el cierre de un `signandsave` que ya se
/// enseñó como firmado.
fn told_of_saving(
    outcome: &crate::site::application::errand::SiteOutcome,
) -> Result<bool, Failure> {
    use crate::site::application::errand::{SiteOutcome, SiteRefusal};
    match outcome {
        SiteOutcome::Refused(SiteRefusal::CannotSaveData(detail)) => {
            Err(Failure::new("cannotSaveData", detail.clone()))
        }
        SiteOutcome::Saved => Ok(true),
        _ => Ok(false),
    }
}

/// Escribe donde la persona eligió, o cancela si cerró el diálogo sin elegir.
fn write_where_chosen(
    chosen: Option<tauri_plugin_dialog::FilePath>,
    consent: &crate::site::application::errand::SavingConsent,
    scratch: &dyn crate::site::ports::Scratch,
    live: &crate::site::application::errand::LiveErrand,
) -> Result<bool, Failure> {
    let Some(chosen) = chosen else {
        crate::site::application::errand::decline(live);
        return Err(save_cancelled());
    };
    let path = named_paths(vec![chosen])?.remove(0).1;
    let outcome = crate::site::application::errand::saved(
        scratch,
        &path,
        &consent.data,
        consent.signer_der.as_deref(),
        live,
    );
    told_of_saving(&outcome)
}

/// Abre el diálogo de guardado del portal y escribe el fichero donde la persona eligió
/// (ADR-0011). `true` cuando hay que enseñar el desenlace de guardado a la ventana.
#[tauri::command(async)]
pub fn site_save_file(
    app_handle: tauri::AppHandle,
    site: State<'_, SiteRoot>,
) -> Result<bool, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let Some(consent) = site.errand.the_saving_pending() else {
        return Err(nothing_pending("ningun guardado"));
    };

    let dialog = save_dialog(app_handle.dialog().file(), &consent);
    write_where_chosen(
        dialog.blocking_save_file(),
        &consent,
        site.scratch.as_ref(),
        &site.errand,
    )
}

/// El selector de carga del portal con las pistas que declaró la sede.
fn load_dialog<R: tauri::Runtime>(
    mut dialog: tauri_plugin_dialog::FileDialogBuilder<R>,
    consent: &crate::site::application::errand::LoadingConsent,
) -> tauri_plugin_dialog::FileDialogBuilder<R> {
    if let Some(name) = consent.filename.as_deref() {
        dialog = dialog.set_file_name(name);
    }
    if let Some(title) = consent.title.as_deref() {
        dialog = dialog.set_title(title);
    }
    if let Some(folder) = consent.starting_folder.as_deref() {
        dialog = dialog.set_directory(folder);
    }
    with_extensions(dialog, &consent.extensions, consent.description.as_deref())
}

/// Elige uno o varios ficheros del selector, según lo que pida la sede.
fn pick<R: tauri::Runtime>(
    dialog: tauri_plugin_dialog::FileDialogBuilder<R>,
    multiple: bool,
) -> Vec<tauri_plugin_dialog::FilePath> {
    if multiple {
        dialog.blocking_pick_files().unwrap_or_default()
    } else {
        dialog.blocking_pick_file().into_iter().collect()
    }
}

/// Traduce en qué quedó la carga a lo que espera la ventana: el paso que sigue si el trámite
/// continúa (`signandsave` con el documento ya elegido), o cuántos ficheros entregó si terminó
/// ahí mismo.
fn told_of_loading(
    completion: crate::site::application::errand::LoadCompletion,
) -> Result<
    (
        Option<crate::site::application::errand::ErrandStep>,
        Option<u32>,
    ),
    Failure,
> {
    use crate::site::application::errand::{LoadCompletion, SiteOutcome, SiteRefusal};
    match completion {
        LoadCompletion::Continues(step) => Ok((Some(step), None)),
        LoadCompletion::Delivered(SiteOutcome::Refused(SiteRefusal::CannotLoadData(detail))) => {
            Err(Failure::new("cannotLoadData", detail))
        }
        // Cualquier otro rechazo ya entregado a la sede: traducción única (ADR-0009).
        LoadCompletion::Delivered(SiteOutcome::Refused(refusal)) => {
            Err(super::frontier::told(&refusal).0)
        }
        LoadCompletion::Delivered(SiteOutcome::Loaded(files)) => {
            Ok((None, Some(files.len() as u32)))
        }
        LoadCompletion::Delivered(_) => Ok((None, None)),
    }
}

/// Lee lo que la persona eligió y continúa el trámite, o cancela si no eligió nada: si el
/// selector esperaba documento para `signandsave`, el paso que sigue no es una entrega a la
/// sede, sino el consentimiento de firma (#494).
fn load_chosen<
    E: crate::site::ports::FilterEngine,
    P: crate::site::ports::PolicyEngine,
    N: crate::site::application::errand::Neighbours,
>(
    chosen: Vec<tauri_plugin_dialog::FilePath>,
    desk: &crate::site::application::errand::ErrandDesk<'_, E, P, N>,
    live: &crate::site::application::errand::LiveErrand,
) -> Result<
    (
        Option<crate::site::application::errand::ErrandStep>,
        Option<u32>,
    ),
    Failure,
> {
    if chosen.is_empty() {
        crate::site::application::errand::decline(live);
        return Err(load_cancelled());
    }
    let named = named_paths(chosen)?;
    told_of_loading(crate::site::application::errand::document_chosen(
        desk, &named, live,
    ))
}

/// Abre el selector de carga del portal y continúa el trámite con lo que la persona eligió
/// (ADR-0011). El valor devuelto es cuántos ficheros entregó a la sede, o `None` si el trámite
/// sigue con un paso más.
#[tauri::command(async)]
pub fn site_load_files(app_handle: tauri::AppHandle) -> Result<Option<u32>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let (moved, delivered) = site_window::with_the_desk(&app_handle, |desk, live| {
        let Some(consent) = live.the_loading_pending() else {
            return Err(nothing_pending("ninguna carga"));
        };

        let dialog = load_dialog(app_handle.dialog().file(), &consent);
        let chosen = pick(dialog, consent.multiple);
        load_chosen(chosen, desk, live)
    })?;
    site_window::publish_what_moved(&app_handle, moved);
    Ok(delivered)
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

#[cfg(test)]
mod tests;
