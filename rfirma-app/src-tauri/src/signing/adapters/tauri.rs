//! Las órdenes de firma local: el ciclo, la previsualización y la configuración.

use tauri::State;

use crate::documents::application::opened::OpenedDocuments;
use crate::signing::adapters::isolate::Isolate;
use crate::Environment;

use super::orders::{PlacementOrder, SigningOrder};
use super::views::ConfigurationView;
use crate::commands::Failure;
use crate::documents::adapters::views::SignedDocumentView;
use crate::identity::adapters::views::SecretView;
use crate::signing::application::session::SigningSession;

/// Prefirma: cruza la frontera y deja el ciclo abierto.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
    opened: State<'_, OpenedDocuments>,
) -> Result<SecretView, Failure> {
    Ok(crate::signing::application::session::begin(
        &order,
        &environment.all_stores(),
        &environment.listed,
        &opened,
        &isolate,
        &session,
    )?
    .into())
}

/// Firma en el token con la clave privada (ADR-0001).
#[tauri::command]
pub fn sign_with_pin(pin: String, session: State<'_, SigningSession>) -> Result<(), Failure> {
    Ok(crate::signing::application::session::sign_on_token(
        &session, &pin,
    )?)
}

/// Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
#[tauri::command]
pub fn finish_signing(
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    session: State<'_, SigningSession>,
) -> Result<SignedDocumentView, Failure> {
    Ok(crate::signing::application::session::finish(
        &isolate,
        &session,
        &environment.memory,
        &environment.configuration(),
        &environment.documents_folder,
    )?
    .into())
}

/// Cancela el ciclo de firma a medias.
#[tauri::command]
pub fn cancel_signing(session: State<'_, SigningSession>) {
    crate::signing::application::session::cancel(&session);
}

/// Previsualización de la firma sobre el PDF sin firmar ni pedir PIN.
#[tauri::command(async)]
pub fn preview_signature(
    order: SigningOrder,
    environment: State<'_, Environment>,
    isolate: State<'_, Isolate>,
    opened: State<'_, OpenedDocuments>,
) -> Result<tauri::ipc::Response, Failure> {
    Ok(tauri::ipc::Response::new(
        crate::signing::application::preview::compose(
            &order,
            &environment.all_stores(),
            &environment.listed,
            &opened,
            &isolate,
        )?,
    ))
}

/// Esquina inferior izquierda del recuadro en puntos PAdES.
#[tauri::command]
pub fn pades_lower_left(placement: PlacementOrder) -> Result<[i32; 2], Failure> {
    let placement = placement.placement()?;
    Ok([placement.rect.lower_left_x, placement.rect.lower_left_y])
}

/// Configuración guardada para la ventana de preferencias.
#[tauri::command]
pub fn read_configuration(environment: State<'_, Environment>) -> ConfigurationView {
    crate::signing::application::configuration::shown(
        &environment.configuration(),
        &environment.documents_folder,
    )
    .into()
}

/// Guarda la configuración elegida por el usuario.
#[tauri::command(async)]
pub fn write_configuration(
    configuration: ConfigurationView,
    environment: State<'_, Environment>,
) -> Result<(), Failure> {
    Ok(crate::signing::application::configuration::write(
        &environment.memory,
        &environment.configuration,
        &configuration.into(),
    )?)
}

/// Olvida los documentos recientes y el certificado usado.
#[tauri::command(async)]
pub fn forget_activity(environment: State<'_, Environment>) -> Result<(), Failure> {
    Ok(crate::signing::application::configuration::forget_activity(
        &environment.memory,
    )?)
}

/// Comprueba si el documento contiene firmas previas no registradas.
#[tauri::command(async)]
pub fn unregistered_signatures(
    document: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<bool, Failure> {
    Ok(crate::signing::application::session::unregistered_signatures_in(&opened, &document)?)
}
