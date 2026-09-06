//! Las órdenes de firma local: el ciclo, la previsualización y la configuración.

use tauri::State;

use crate::app::{self, Environment};
use crate::isolate::Isolate;
use crate::memory::OpenedDocuments;

use super::{
    ConfigurationView, Failure, PlacementOrder, SecretView, SignedDocumentView, SigningOrder,
    SigningSession,
};

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

/// Comprueba si el documento contiene firmas previas no registradas.
#[tauri::command(async)]
pub fn unregistered_signatures(
    document: String,
    opened: State<'_, OpenedDocuments>,
) -> Result<bool, Failure> {
    app::signing::unregistered_signatures_in(&opened, &document)
}
