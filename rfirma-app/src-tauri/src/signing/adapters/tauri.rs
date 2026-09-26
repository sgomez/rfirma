//! Las órdenes de firma local: el ciclo, la previsualización y la configuración.

use tauri::State;

use crate::documents::DocumentsRoot;
use crate::identity::IdentityRoot;
use crate::signing::SigningRoot;

use super::memory::Memory;
use super::orders::{PlacementOrder, SigningOrder};
use super::views::{ConfigurationView, RememberedVisibleSignatureView};
use crate::crossing::Failure;
use crate::documents::adapters::views::SignedDocumentView;
use crate::identity::adapters::views::SecretView;
use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::application::session::{self, DocumentToSign};
use crate::signing::domain::config::SigningChoice;
use crate::signing::domain::VisibleContent;

/// Prefirma: cruza la frontera y deja el ciclo abierto.
#[tauri::command]
pub fn begin_signing(
    order: SigningOrder,
    identity: State<'_, IdentityRoot>,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<SecretView, Failure> {
    let (document, chosen, choice) = what_is_ordered(&order, &identity, &documents)?;
    remember_the_visible_signature_ordered(&order, &signing.memory);
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

/// El modelo, la frase y «Con rúbrica» de la orden se recuerdan en la prefirma, antes de abrir el ciclo.
fn remember_the_visible_signature_ordered(order: &SigningOrder, memory: &Memory) {
    if let Some(content) = &order.content {
        let content = VisibleContent::from(content);
        let _ = memory.remember_visible_signature(Some(&content), order.with_rubric);
    }
}

/// Firma en el token con la clave privada (ADR-0001).
#[tauri::command(async)]
pub fn sign_with_pin(pin: String, app_handle: tauri::AppHandle) -> Result<(), Failure> {
    crate::site::adapters::window::with_the_desk(&app_handle, |desk, live| {
        signed_with_the_secret(desk, live, &pin)
    })
}

/// La única puerta del PIN: cierra el lote consentido o la firma contra el servidor trifásico, o firma el ciclo abierto.
pub fn signed_with_the_secret(
    desk: &crate::site::SiteDesk<'_>,
    live: &crate::site::LiveErrand,
    pin: &str,
) -> Result<(), Failure> {
    let signer = desk.neighbours.identity.signer();
    let signing = desk.neighbours.signing;
    let prompter = signing.prompter.as_ref();
    let language = signing.configuration().language;

    let Some(certificate) = live.the_certificate_awaiting_the_secret() else {
        return Ok(session::signed_on_the_token(
            &signer,
            &signing.session,
            prompter,
            language,
            pin,
        )?);
    };
    let secret = session::secret_for_the_batch(&signer, &certificate, prompter, language, pin)?;
    crate::site::the_pending_signature_signed(desk, live, &secret)
}

/// Postfirma: comprueba el sello, ensambla el PDF y lo deja caer donde se eligió.
#[tauri::command]
pub fn finish_signing(
    destination: Option<String>,
    identity: State<'_, IdentityRoot>,
    documents: State<'_, DocumentsRoot>,
    signing: State<'_, SigningRoot>,
) -> Result<SignedDocumentView, Failure> {
    let signed = crate::signing::application::session::finish(&signing.isolate, &signing.session)?;
    let (landing, delivered) = documents.deliver(
        &signed.document,
        signed.completed.signed_document(),
        destination.as_deref(),
    )?;
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

/// Modelo, frase y «Con rúbrica» de la última firma visible configurada (ADR-0010).
#[tauri::command]
pub fn remembered_visible_signature(
    signing: State<'_, SigningRoot>,
) -> RememberedVisibleSignatureView {
    signing.memory.remembered_visible_signature().into()
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

#[cfg(test)]
mod tests;
