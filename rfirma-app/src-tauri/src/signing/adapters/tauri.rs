//! Las órdenes de firma local: el ciclo, la previsualización y la configuración.

use tauri::State;

use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::signing::SigningRoot;

use super::orders::{PlacementOrder, SigningOrder};
use super::views::ConfigurationView;
use crate::crossing::Failure;
use crate::documents::adapters::views::SignedDocumentView;
use crate::identity::adapters::views::SecretView;
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::session::DocumentToSign;
use crate::signing::domain::config::SigningChoice;

/// Prefirma: cruza la frontera y deja el ciclo abierto.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    identity: State<'_, IdentityRoot>,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<SecretView, Failure> {
    let (document, chosen, choice) = what_is_ordered(&order, &identity, &documents)?;
    Ok(crate::signing::application::session::begin(
        signing.files.as_ref(),
        document,
        &chosen,
        &choice,
        &identity.signer(),
        &signing.isolate,
        &signing.session,
    )?
    .into())
}

/// Firma en el token con la clave privada (ADR-0001).
#[tauri::command]
pub fn sign_with_pin(
    pin: String,
    identity: State<'_, IdentityRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<(), Failure> {
    Ok(crate::signing::application::session::sign_on_token(
        &identity.signer(),
        &signing.session,
        &pin,
    )?)
}

/// Postfirma: comprueba el sello, ensambla el PDF y lo deja caer.
#[tauri::command]
pub fn finish_signing(
    identity: State<'_, IdentityRoot>,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<SignedDocumentView, Failure> {
    let signed = crate::signing::application::session::finish(&signing.isolate, &signing.session)?;
    let (landing, delivered) = documents.deliver(&signed.document, signed.completed.pdf())?;
    identity.remember_the_certificate(&signed.certificate);
    if documents.is_remembered(&signed.handle) {
        documents.note_signed(&landing, &signed.completed);
    }
    crate::signing::application::session::note_delivered(&signing.session, landing);
    Ok(delivered.into())
}

/// Cancela el ciclo de firma a medias.
#[tauri::command]
pub fn cancel_signing(signing: State<'_, SigningRoot>) {
    crate::signing::application::session::cancel(&signing.session);
}

/// Previsualización de la firma sobre el PDF sin firmar ni pedir PIN.
#[tauri::command(async)]
pub fn preview_signature(
    order: SigningOrder,
    identity: State<'_, IdentityRoot>,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<tauri::ipc::Response, Failure> {
    let (document, chosen, choice) = what_is_ordered(&order, &identity, &documents)?;
    Ok(tauri::ipc::Response::new(
        crate::signing::application::preview::compose(
            signing.files.as_ref(),
            &document.document,
            &chosen,
            &choice,
            &signing.isolate,
        )?,
    ))
}

fn what_is_ordered(
    order: &SigningOrder,
    identity: &IdentityRoot,
    documents: &DocumentsRoot,
) -> Result<(DocumentToSign, TokenCertificate, SigningChoice), Failure> {
    let document = documents.opened_document(&order.document)?;
    let chosen = identity.chosen(&order.certificate)?;
    Ok((
        DocumentToSign {
            handle: order.document.clone(),
            document,
        },
        chosen,
        order.choice()?,
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
pub fn read_configuration(
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> ConfigurationView {
    crate::signing::application::configuration::shown(
        &signing.configuration(),
        &documents.documents_folder,
    )
    .into()
}

/// Guarda la configuración elegida por el usuario.
#[tauri::command(async)]
pub fn write_configuration(
    configuration: ConfigurationView,
    signing: State<'_, SigningRoot>,
) -> Result<(), Failure> {
    let next = crate::signing::application::configuration::merged(
        &signing.configuration(),
        &configuration.into(),
    );
    Ok(signing.memory.remember_configuration(&next)?)
}

/// Olvida los documentos recientes y el certificado usado.
#[tauri::command(async)]
pub fn forget_activity(signing: State<'_, SigningRoot>) -> Result<(), Failure> {
    Ok(signing.memory.forget_activity()?)
}

/// Comprueba si el documento contiene firmas previas no registradas.
#[tauri::command(async)]
pub fn unregistered_signatures(
    document: String,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<bool, Failure> {
    let document = documents.opened_document(&document)?;
    Ok(
        crate::signing::application::session::unregistered_signatures_in(
            signing.files.as_ref(),
            &document,
        )?,
    )
}
