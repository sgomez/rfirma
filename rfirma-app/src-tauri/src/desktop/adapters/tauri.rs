//! Las órdenes del escritorio: invocación, versión publicada y manejadores afirma://.

use tauri::State;

use crate::desktop::DesktopRoot;
use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::site::SiteRoot;

use super::registry::DesktopRegistry;
use super::views::{NewVersionView, SignalRowView, UrlHandlersView};
use crate::crossing::Failure;
use crate::desktop::domain::error::{DesktopError, Situation};
use crate::desktop::domain::status::{StoreBrand, StoreDetail};
use crate::documents::adapters::views::DroppedDocumentView;
use crate::identity::domain::store::{Store, StoreClass};

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    desktop: State<'_, DesktopRoot>,
    documents: State<'_, DocumentsRoot>,
) -> Option<DroppedDocumentView> {
    let invocation = desktop.pending_invocation.take()?;
    let paths = crate::desktop::application::invocation::invoked_documents(&invocation)?;
    documents
        .what_was_dropped(&paths)
        .map(DroppedDocumentView::from)
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(desktop: State<'_, DesktopRoot>) -> Option<NewVersionView> {
    let announced = crate::desktop::application::version::new_version(
        crate::desktop::application::version::Version::running(),
        desktop.memory.as_ref(),
        &crate::desktop::adapters::releases::latest_release,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.to_string(),
    })
}

/// Manejadores registrados para el esquema afirma:// en el escritorio (ADR-0015).
#[tauri::command(async)]
pub fn url_handlers() -> UrlHandlersView {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list =
        crate::desktop::adapters::choice::mimeapps_list_from_environment().unwrap_or_default();
    crate::desktop::application::handlers::who_handles(&DesktopRegistry::of(channel, list)).into()
}

/// Establece el manejador preferido para el esquema afirma:// (ADR-0015).
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list = crate::desktop::adapters::choice::mimeapps_list_from_environment()
        .map_err(|error| DesktopError::new(Situation::TheListIsNotWritable, error.to_string()))?;
    Ok(crate::desktop::application::handlers::chosen(
        &DesktopRegistry::of(channel, list),
        &handler,
    )?)
}

/// Abre un destino externo por identificador en el navegador del sistema (ADR-0011).
#[tauri::command(async)]
pub fn open_external_destination(
    app_handle: tauri::AppHandle,
    target: String,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    crate::desktop::application::destination::open_destination(&target, |url| {
        app_handle
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| error.to_string())
    })
    .map_err(|error| Failure::new("unknownDestination", error.to_string()))
}

/// Marca del almacén NSS de un perfil, para el detalle de la señal del certificado de rFirma.
fn brand_of(profile: &std::path::Path) -> StoreBrand {
    match Store::nss(std::path::PathBuf::new(), profile).class() {
        StoreClass::Firefox => StoreBrand::Firefox,
        StoreClass::Chrome => StoreBrand::Chrome,
        StoreClass::Nssdb | StoreClass::Card | StoreClass::Installed => StoreBrand::Nssdb,
    }
}

/// Mide la señal del certificado de rFirma, o la deja en «Comprobando» si no se pide remedir.
fn local_ca_certificate_signal(
    site: &SiteRoot,
    recheck: bool,
) -> crate::desktop::domain::status::SignalRow {
    if !recheck {
        return crate::desktop::application::status::checking_local_ca_certificate_signal();
    }
    measured_local_ca_certificate_signal(site, false)
}

/// Mide la señal del certificado de rFirma, con el aviso de reiniciar Firefox si se pide.
fn measured_local_ca_certificate_signal(
    site: &SiteRoot,
    restart_firefox_notice: bool,
) -> crate::desktop::domain::status::SignalRow {
    let detail = site
        .measure_local_ca_trust()
        .unwrap_or_default()
        .into_iter()
        .map(|reading| StoreDetail {
            brand: brand_of(&reading.profile),
            trusted: reading.trusted,
        })
        .collect();
    crate::desktop::application::status::evaluate_local_ca_certificate_signal(
        detail,
        restart_firefox_notice,
    )
}

/// Si Firefox tiene abierto alguno de los perfiles detectados.
fn firefox_is_running(site: &SiteRoot) -> bool {
    site.trust
        .profiles
        .iter()
        .filter(|profile| brand_of(profile) == StoreBrand::Firefox)
        .any(|profile| crate::desktop::adapters::firefox_lock::firefox_is_running(profile))
}

/// Si la CA local vigente es de confianza en cada perfil Firefox detectado.
fn firefox_local_ca_trust(site: &SiteRoot) -> Vec<bool> {
    site.measure_local_ca_trust()
        .unwrap_or_default()
        .into_iter()
        .filter(|reading| brand_of(&reading.profile) == StoreBrand::Firefox)
        .map(|reading| reading.trusted)
        .collect()
}

/// Instala el certificado de rFirma donde falte y vuelve a medir la señal.
#[tauri::command(async)]
pub fn install_local_ca_certificate(site: State<'_, SiteRoot>) -> SignalRowView {
    let firefox_was_running = firefox_is_running(&site);
    let firefox_trusted_before = firefox_local_ca_trust(&site);
    let _ = site.install_local_ca_trust();
    let firefox_trusted_after = firefox_local_ca_trust(&site);
    let restart_firefox_notice = crate::desktop::application::status::firefox_restart_notice(
        firefox_was_running,
        &firefox_trusted_before,
        &firefox_trusted_after,
    );
    measured_local_ca_certificate_signal(&site, restart_firefox_notice).into()
}

/// Elige quién abre las sedes; si es rFirma, instala también su certificado (ADR-0009).
#[tauri::command(async)]
pub fn choose_site_signature_handler(
    handler: String,
    site: State<'_, SiteRoot>,
) -> Result<Vec<SignalRowView>, Failure> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list = crate::desktop::adapters::choice::mimeapps_list_from_environment()
        .map_err(|error| DesktopError::new(Situation::TheListIsNotWritable, error.to_string()))?;
    let registry = DesktopRegistry::of(channel, list);
    crate::desktop::application::handlers::chosen(&registry, &handler)?;

    let firefox_was_running = firefox_is_running(&site);
    let firefox_trusted_before = firefox_local_ca_trust(&site);
    if handler == crate::desktop::domain::handlers::OUR_DESKTOP_FILE {
        let _ = site.install_local_ca_trust();
    }
    let firefox_trusted_after = firefox_local_ca_trust(&site);
    let restart_firefox_notice = crate::desktop::application::status::firefox_restart_notice(
        firefox_was_running,
        &firefox_trusted_before,
        &firefox_trusted_after,
    );

    let handlers = crate::desktop::application::handlers::who_handles(&registry);
    Ok(vec![
        crate::desktop::application::status::evaluate_site_signature_signal(handlers).into(),
        measured_local_ca_certificate_signal(&site, restart_firefox_notice).into(),
    ])
}

/// Consulta el estado de las señales de la instalación para el panel de estado.
#[tauri::command(async)]
pub fn read_status(
    desktop: State<'_, DesktopRoot>,
    identity: State<'_, IdentityRoot>,
    site: State<'_, SiteRoot>,
    recheck: bool,
) -> Vec<SignalRowView> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let list =
        crate::desktop::adapters::choice::mimeapps_list_from_environment().unwrap_or_default();
    let handlers =
        crate::desktop::application::handlers::who_handles(&DesktopRegistry::of(channel, list));
    vec![
        crate::desktop::application::status::check_version_signal(
            crate::desktop::application::version::Version::running(),
            desktop.memory.as_ref(),
            &crate::desktop::adapters::releases::latest_release,
            channel,
            recheck,
            std::time::SystemTime::now(),
        )
        .into(),
        crate::desktop::application::status::evaluate_site_signature_signal(handlers).into(),
        local_ca_certificate_signal(&site, recheck).into(),
        crate::desktop::application::status::evaluate_user_certificates_signal(
            identity.stores_with_certificates(),
        )
        .into(),
    ]
}
