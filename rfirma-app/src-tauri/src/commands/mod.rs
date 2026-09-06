//! Las órdenes de Tauri: lo único que la ventana puede pedir. No deciden nada.

pub mod failure;
pub mod orders;
pub mod rubric;
pub mod site_window;
pub mod views;
pub mod views_site;

#[cfg(test)]
mod guards;

use tauri::State;

use crate::app::{self, Environment};
use crate::isolate::Isolate;
use crate::memory::OpenedDocuments;

pub use crate::app::invocation::PendingInvocation;
pub use crate::app::signing::SigningSession;
pub use app::documents::dropped_document;
pub use app::invocation::second_invocation;
pub use failure::Failure;
pub use orders::{PlacementOrder, SigningOrder};
pub use rubric::{RubricChoiceView, RubricView};
pub use site_window::{
    attend_site_operation, open_the_site_window, publish_the_moment, SITE_ERRAND, SITE_WINDOW,
};
pub use views::{
    CertificateView, ConfigurationView, DestinationView, DroppedDocumentView, NewVersionView,
    OpenedDocumentView, PlacementView, RecentDocumentView, SecretView, SignedDocumentView,
    UrlHandlerView, UrlHandlersView,
};
pub use views_site::{
    NoCertificateView, NoChannelView, RefusalSituationView, SignatureRoundView, SiteErrandView,
    SiteOutcomeView, SiteStageView,
};

/// Certificados de los tokens conectados.
#[tauri::command]
pub fn list_certificates(
    environment: State<'_, Environment>,
) -> Result<Vec<CertificateView>, Failure> {
    app::certificates::listed_rows(
        &environment.all_stores(),
        &environment.installed_certificates,
        &environment.listed,
        &environment.memory,
    )
}

/// Prefirma: cruza la frontera y deja el ciclo abierto.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
) -> Result<SecretView, Failure> {
    app::signing::begin(
        &order,
        &environment.all_stores(),
        &environment.listed,
        &opened,
        &isolate,
        &session,
    )
    .map(SecretView::from)
}

/// Firma en el token con la clave privada (ADR-0001).
#[tauri::command]
pub fn sign_with_pin(pin: String, session: State<'_, SigningSession>) -> Result<(), Failure> {
    app::signing::sign_on_token(&session, &pin)
}

/// Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
#[tauri::command]
pub fn finish_signing(
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
) -> Result<SignedDocumentView, Failure> {
    app::signing::finish(
        &isolate,
        &session,
        &environment.memory,
        &environment.configuration(),
        &environment.documents_folder,
    )
}

/// Cancela el ciclo de firma a medias.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    app::signing::cancel(&session);
}

/// Abre el diálogo del sistema y apunta lo que el portal conceda.
#[tauri::command(async)]
pub fn open_document(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<Option<OpenedDocumentView>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let configuration = environment.configuration();
    let mut dialog = app_handle.dialog().file().add_filter("PDF", &["pdf"]);
    if let Some(folder) = app::documents::starting_folder(
        &environment.memory,
        &configuration,
        &environment.documents_folder,
    ) {
        dialog = dialog.set_directory(folder);
    }
    let Some(chosen) = dialog.blocking_pick_file() else {
        return Ok(None);
    };
    let handle = chosen
        .into_path()
        .map_err(|error| Failure::new("documentUnreadable", error.to_string()))?;
    Ok(Some(app::documents::note_opened(
        &environment.memory,
        &configuration,
        &opened,
        handle,
    )))
}

/// Los bytes del documento abierto.
#[tauri::command(async)]
pub fn read_document(
    id: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(app::documents::bytes_of(
        &opened, &id,
    )?))
}

/// Configuración guardada para la ventana de preferencias.
#[tauri::command]
pub fn read_configuration(environment: State<'_, Environment>) -> ConfigurationView {
    app::configuration::shown(&environment.configuration(), &environment.documents_folder)
}

/// Guarda la configuración elegida por el usuario.
#[tauri::command(async)]
pub fn write_configuration(
    configuration: ConfigurationView,
    environment: State<'_, Environment>,
) -> Result<(), Failure> {
    app::configuration::write(
        &environment.memory,
        &environment.configuration,
        &configuration,
    )
}

/// Olvida los documentos recientes y el certificado usado.
#[tauri::command(async)]
pub fn forget_activity(environment: State<'_, Environment>) -> Result<(), Failure> {
    app::configuration::forget_activity(&environment.memory)
}

/// **Orden 11.** La bandeja entera, la más reciente primero.
///
/// Bandeja de documentos recientes (ADR-0010).
#[tauri::command(async)]
pub fn list_recents(
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Vec<RecentDocumentView> {
    app::recents::listed_rows(&environment.memory, &opened)
}

/// Anota en la bandeja el documento abierto y su recuadro.
#[tauri::command(async)]
pub fn record_recent(
    id: String,
    placement: Option<PlacementView>,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<RecentDocumentView, Failure> {
    app::in_hand::take(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
        placement,
    )
}

/// Quita una fila de la bandeja de recientes.
#[tauri::command(async)]
pub fn forget_recent(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<(), Failure> {
    app::recents::forget(
        &environment.memory,
        &environment.configuration(),
        &opened,
        &id,
    )
}

/// Abre el diálogo del portal y adopta la imagen elegida como rúbrica (ADR-0012).
#[tauri::command(async)]
pub fn choose_rubric(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Option<RubricChoiceView> {
    use tauri_plugin_dialog::DialogExt;

    let dialog = app_handle
        .dialog()
        .file()
        .add_filter("Imagen", &["png", "jpg", "jpeg"]);
    let chosen = dialog.blocking_pick_file()?;
    Some(match app::rubric::choose(&environment.rubric, chosen) {
        Ok(normalized) => RubricChoiceView::adopted(&normalized),
        Err(error) => RubricChoiceView::refused(&error),
    })
}

/// La rúbrica adoptada si la hay (ADR-0012).
#[tauri::command(async)]
pub fn read_rubric(environment: State<'_, Environment>) -> Result<Option<RubricView>, Failure> {
    let stored = app::rubric::stored(&environment.rubric)?;
    Ok(stored.map(|bytes| RubricView::from_bytes(&bytes)))
}

/// Destino previsto para el documento antes de firmar.
#[tauri::command(async)]
pub fn preview_destination(
    id: String,
    environment: State<'_, Environment>,
    opened: State<'_, OpenedDocuments>,
) -> Result<DestinationView, Failure> {
    let document = app::documents::opened_document(&opened, &id)?;
    Ok(app::documents::where_it_lands(
        &environment.configuration(),
        &environment.documents_folder,
        &document,
    ))
}

/// Abre el selector de directorio y guarda la carpeta de destino elegida (ADR-0011).
#[tauri::command(async)]
pub fn choose_destination(
    app_handle: tauri::AppHandle,
    environment: State<'_, Environment>,
) -> Result<Option<String>, Failure> {
    use tauri_plugin_dialog::DialogExt;

    let Some(chosen) = app_handle.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = chosen
        .into_path()
        .map_err(|error| Failure::new("folderMissing", error.to_string()))?;
    app::configuration::choose_destination(
        &environment.memory,
        &environment.configuration,
        crate::destination::DestinationFolder::at(folder),
    )
    .map(Some)
}

/// Abre el PDF firmado con el visor del sistema (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_document(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let landing = app::signing::signed_document(&session)?;
    app_handle
        .opener()
        .open_path(landing.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// Abre la carpeta donde quedó el PDF firmado (ADR-0011).
#[tauri::command(async)]
pub fn open_signed_folder(
    app_handle: tauri::AppHandle,
    session: State<'_, SigningSession>,
) -> Result<(), Failure> {
    use tauri_plugin_opener::OpenerExt;

    let folder = app::signing::signed_folder(&session)?;
    app_handle
        .opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|error| Failure::new("unknown", error.to_string()))
}

/// Previsualización de la firma sobre el PDF sin firmar ni pedir PIN.
#[tauri::command(async)]
pub fn preview_signature(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(app::preview::compose(
        &order,
        &environment.all_stores(),
        &environment.listed,
        &opened,
        &isolate,
    )?))
}

/// Esquina inferior izquierda del recuadro en puntos PAdES.
#[tauri::command]
pub fn pades_lower_left(placement: PlacementOrder) -> Result<[i32; 2], Failure> {
    let placement = placement.placement()?;
    Ok([placement.rect.lower_left_x, placement.rect.lower_left_y])
}

/// Documento con el que se invocó la aplicación si lo hubo.
#[tauri::command]
pub fn read_invocation(
    pending: State<'_, PendingInvocation>,
    opened: State<'_, OpenedDocuments>,
) -> Option<DroppedDocumentView> {
    let invocation = pending.take()?;
    app::invocation::invoked_document(&invocation, &opened)
}

/// Comprueba si hay una versión nueva publicada.
#[tauri::command(async)]
pub fn check_for_new_version(environment: State<'_, Environment>) -> Option<NewVersionView> {
    let announced = app::version::new_version(
        app::version::Version::running(),
        &environment.memory,
        &crate::releases::latest_release,
        std::time::SystemTime::now(),
    )?;

    Some(NewVersionView {
        version: announced.to_string(),
    })
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

    app::certificates::install_pkcs12(&environment.installed_certificates, chosen, &password)
        .map(|()| true)
}

/// Desinstala un certificado PKCS#12 previamente instalado.
#[tauri::command(async)]
pub fn remove_certificate(id: String, environment: State<'_, Environment>) -> Result<(), Failure> {
    app::certificates::remove_installed(
        &environment.installed_certificates,
        &id,
        &environment.listed,
    )
}

/// Manejadores registrados para el esquema afirma:// en el escritorio (ADR-0015).
#[tauri::command(async)]
pub fn url_handlers() -> UrlHandlersView {
    let channel = crate::desktop::Channel::detected();
    let list = crate::desktop::choice::mimeapps_list_from_environment().unwrap_or_default();
    app::handlers::who_handles(channel, &list)
}

/// Establece el manejador preferido para el esquema afirma:// (ADR-0015).
#[tauri::command(async)]
pub fn choose_url_handler(handler: String) -> Result<(), Failure> {
    let channel = crate::desktop::Channel::detected();
    let list = crate::desktop::choice::mimeapps_list_from_environment().map_err(|error| {
        Failure::new(
            app::handlers::situation_name(crate::desktop::error::Situation::TheListIsNotWritable),
            error.to_string(),
        )
    })?;
    app::handlers::chosen(channel, &list, &handler)
}

/// Comprueba si el documento contiene firmas previas no registradas.
#[tauri::command(async)]
pub fn unregistered_signatures(
    document: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<bool, Failure> {
    app::signing::unregistered_signatures_in(&opened, &document)
}

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
        app::errand::consent(desk, &certificate, live)
    })
    .map(|_| ())
}

/// Cancela el trámite ante la sede.
#[tauri::command(async)]
pub fn site_decline(live: State<'_, app::errand::LiveErrand>) {
    app::errand::decline(&live);
}

/// Inicia la firma del trámite de sede con el certificado elegido (ADR-0001).
#[tauri::command(async)]
pub fn site_begin_signing(
    certificate: String,
    app_handle: tauri::AppHandle,
) -> Result<SecretView, Failure> {
    match site_window::with_the_desk(&app_handle, |desk, live| {
        app::errand::consent(desk, &certificate, live)
    })? {
        app::errand::Consented::SigningWith(secret) => Ok(SecretView::from(secret)),
        app::errand::Consented::IdentityHandedOver => Err(Failure::new(
            "siteErrandNotLive",
            "lo que habia pendiente era una identificacion, y ya se ha entregado",
        )),
    }
}

/// Postfirma del trámite de sede y entrega del resultado a la sede.
#[tauri::command(async)]
pub fn site_finish_signing(app_handle: tauri::AppHandle) -> Result<(), Failure> {
    site_window::with_the_desk(&app_handle, app::errand::finish)
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
        app::errand::look_again(desk, live)
    });
    site_window::publish_what_moved(&app_handle, looked);
}

/// Instala la CA local en los almacenes NSS del usuario (ADR-0005).
#[tauri::command(async)]
pub fn install_local_ca(
    app_handle: tauri::AppHandle,
    trust: State<'_, app::startup::LocalCaTrust>,
    held: State<'_, app::startup::HeldChannel>,
    live: State<'_, app::errand::LiveErrand>,
) {
    app::startup::repair_the_local_ca(&trust, &held, &live);
    site_window::publish_the_moment(&app_handle);
}

/// Consulta el momento actual del trámite de sede.
#[tauri::command]
pub fn read_site_errand(live: State<'_, app::errand::LiveErrand>) -> Option<SiteErrandView> {
    live.moment().as_ref().map(SiteErrandView::from)
}

/// Nombre del evento emitido cuando se suelta un documento en la ventana.
pub const DOCUMENT_DROPPED: &str = "document-dropped";

#[cfg(test)]
mod tests {
    use super::{pades_lower_left, PlacementOrder};

    #[test]
    fn matches_user_space_when_the_page_is_not_rotated() {
        let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
            "page": 1,
            "pages": { "only": [1] },
            "pageCount": 1,
            "mediaBox": [0.0, 0.0, 595.0, 842.0],
            "rotation": 0,
            "rect": [250.0, 50.0, 450.0, 100.0],
        }))
        .expect("la orden del visor");

        assert_eq!(
            pades_lower_left(placement).expect("cabe en la pagina"),
            [250, 50]
        );
    }

    #[test]
    fn diverges_from_user_space_when_the_page_is_rotated() {
        let placement: PlacementOrder = serde_json::from_value(serde_json::json!({
            "page": 1,
            "pages": { "only": [1] },
            "pageCount": 1,
            "mediaBox": [0.0, 0.0, 595.0, 842.0],
            "rotation": 90,
            "rect": [250.0, 50.0, 450.0, 100.0],
        }))
        .expect("la orden del visor");

        assert_eq!(
            pades_lower_left(placement).expect("cabe en la pagina"),
            [50, 145]
        );
    }
}
