//! Las órdenes del escritorio: invocación, barra de título nativa, versión publicada, manejadores afirma:// y su elección, destino externo, panel de estado y retirada.

use tauri::State;

use crate::desktop::DesktopRoot;
use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::site::SiteRoot;

use super::views::{
    InstallationView, InvokedDocumentView, NewVersionView, SignalRowView, TitlebarStateView,
    WithdrawalReportView,
};
use crate::crossing::Failure;
use crate::desktop::domain::command_line::WindowIntent;
use crate::desktop::domain::status::{StoreBrand, StoreCertificates, StoreDetail};
use crate::documents::adapters::views::DroppedDocumentView;
use crate::identity::domain::store::{Store, StoreClass};

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    desktop: State<'_, DesktopRoot>,
    documents: State<'_, DocumentsRoot>,
) -> Option<InvokedDocumentView> {
    let invocation = desktop.pending_invocation.take()?;
    let paths = crate::desktop::application::invocation::invoked_documents(&invocation)?;
    let intent = crate::desktop::application::invocation::invoked_intent(&invocation);
    invoked_document(&documents, &paths, intent)
}

/// Lo que se abrió de esas rutas invocadas, con su intención.
pub fn invoked_document(
    documents: &DocumentsRoot,
    paths: &[std::path::PathBuf],
    intent: WindowIntent,
) -> Option<InvokedDocumentView> {
    let told = match intent {
        WindowIntent::SeeItsSignatures => {
            documents.what_was_handed_over_to_see_its_signatures(paths)
        }
        WindowIntent::OpenTheDocument => documents.what_was_dropped(paths),
    };
    told.map(|opened| InvokedDocumentView {
        opened: DroppedDocumentView::from(opened),
        intent,
    })
}

/// Aplica en el hilo principal el estado de la barra de título nativa que manda la ventana.
#[tauri::command(async)]
pub fn apply_titlebar_state(app_handle: tauri::AppHandle, state: TitlebarStateView) {
    let _ = app_handle.run_on_main_thread(move || super::titlebar::apply(&state));
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(desktop: State<'_, DesktopRoot>) -> Option<NewVersionView> {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    let announced = crate::desktop::application::version::new_version(
        crate::desktop::application::version::Version::running(),
        desktop.memory.as_ref(),
        &|| crate::desktop::adapters::releases::latest_release(channel),
        channel,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.version.to_string(),
        installable: announced.installable,
    })
}

/// Instala la versión anunciada: la descarga, verifica su firma, cierra la aplicación y lanza el instalador.
#[tauri::command(async)]
pub fn install_new_version(app_handle: tauri::AppHandle) -> InstallationView {
    let installer = crate::desktop::adapters::installer::this_desktop(app_handle);
    crate::desktop::application::version::install_new_version(
        crate::desktop::application::version::Version::running(),
        installer.as_ref(),
    )
    .into()
}

/// Mide la señal de versión consultando GitHub.
#[tauri::command(async)]
pub fn measure_version(desktop: State<'_, DesktopRoot>) -> SignalRowView {
    let channel = crate::desktop::adapters::channel::Channel::detected();
    crate::desktop::application::status::measure_version_signal(
        crate::desktop::application::version::Version::running(),
        desktop.memory.as_ref(),
        &|| crate::desktop::adapters::releases::latest_release(channel),
        channel,
        std::time::SystemTime::now(),
    )
    .into()
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

/// Marca del almacén de confianza de un perfil, para el detalle de la señal del certificado de rFirma.
fn brand_of(profile: &std::path::Path) -> StoreBrand {
    if crate::site::adapters::is_the_system_root_store(profile) {
        return StoreBrand::Windows;
    }
    match Store::nss(std::path::PathBuf::new(), profile).class() {
        StoreClass::Firefox => StoreBrand::Firefox,
        StoreClass::Chrome => StoreBrand::Chrome,
        StoreClass::Nssdb | StoreClass::Card | StoreClass::Installed | StoreClass::Windows => {
            StoreBrand::Nssdb
        }
    }
}

/// Marca del sitio donde hay certificados propios, que puede ser una tarjeta o un `.p12`.
fn brand_of_class(class: StoreClass) -> StoreBrand {
    match class {
        StoreClass::Firefox => StoreBrand::Firefox,
        StoreClass::Chrome => StoreBrand::Chrome,
        StoreClass::Nssdb => StoreBrand::Nssdb,
        StoreClass::Card => StoreBrand::Card,
        StoreClass::Installed => StoreBrand::Installed,
        StoreClass::Windows => StoreBrand::Windows,
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

/// Mide la señal del certificado de rFirma, que nace en «Comprobando» al leer el estado.
#[tauri::command(async)]
pub fn measure_local_ca_certificate(site: State<'_, SiteRoot>) -> SignalRowView {
    measured_local_ca_certificate_signal(&site, false).into()
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
    let registry = crate::desktop::adapters::registry::this_desktop_to_write()?;
    crate::desktop::application::handlers::chosen(registry.as_ref(), &handler)?;

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

    let handlers = crate::desktop::application::handlers::who_handles(registry.as_ref());
    Ok(vec![
        crate::desktop::application::status::evaluate_site_signature_signal(
            handlers,
            crate::desktop::adapters::channel::Channel::detected(),
        )
        .into(),
        measured_local_ca_certificate_signal(&site, restart_firefox_notice).into(),
    ])
}

/// Consulta el estado de las señales de la instalación para el panel de estado.
#[tauri::command(async)]
pub fn read_status(
    identity: State<'_, IdentityRoot>,
    site: State<'_, SiteRoot>,
    recheck: bool,
) -> Vec<SignalRowView> {
    let handlers = crate::desktop::application::handlers::who_handles(
        crate::desktop::adapters::registry::this_desktop().as_ref(),
    );
    vec![
        crate::desktop::application::status::checking_version_signal().into(),
        crate::desktop::application::status::evaluate_site_signature_signal(
            handlers,
            crate::desktop::adapters::channel::Channel::detected(),
        )
        .into(),
        local_ca_certificate_signal(&site, recheck).into(),
        crate::desktop::application::status::evaluate_user_certificates_signal(
            identity
                .certificates_by_class()
                .into_iter()
                .map(|(class, certificates)| StoreCertificates {
                    brand: brand_of_class(class),
                    certificates,
                })
                .collect(),
        )
        .into(),
    ]
}

/// Retira lo que rFirma dejó fuera de sus carpetas: el manejador de sedes y la CA local de cada
/// almacén NSS. Con `previous`, `Reintentar` solo vuelve a tocar lo que en él falló.
#[tauri::command(async)]
pub fn withdraw_rfirma(
    previous: Option<WithdrawalReportView>,
    site: State<'_, SiteRoot>,
) -> WithdrawalReportView {
    use crate::desktop::application::withdrawal::{
        handler_needs_retry, merged_report, profiles_to_retry,
    };
    use crate::desktop::domain::withdrawal::{Withdrawal, WithdrawalReport};

    let registry = crate::desktop::adapters::registry::this_desktop();
    let previous = previous.map(WithdrawalReport::from);

    let profiles: Vec<(std::path::PathBuf, StoreBrand)> = site
        .nss_profiles()
        .iter()
        .map(|profile| (profile.clone(), brand_of(profile)))
        .collect();

    let handler = if handler_needs_retry(previous.as_ref()) {
        crate::desktop::application::handlers::withdrawn(registry.as_ref())
    } else {
        previous
            .as_ref()
            .map(|report| report.handler.clone())
            .unwrap_or(Withdrawal::WasNotThere)
    };

    let retry = profiles_to_retry(&profiles, previous.as_ref());
    let retried = site
        .withdraw_local_ca_trust(&retry)
        .map(|outcome| {
            outcome
                .results
                .into_iter()
                .map(|(profile, outcome)| {
                    let outcome = match outcome.failure() {
                        Some(reason) => Withdrawal::Failed(reason),
                        None if outcome.withdrawn() => Withdrawal::Withdrawn,
                        None => Withdrawal::WasNotThere,
                    };
                    (profile, outcome)
                })
                .collect()
        })
        .unwrap_or_default();

    merged_report(handler, &profiles, retried, previous.as_ref()).into()
}

#[cfg(test)]
mod tests;
