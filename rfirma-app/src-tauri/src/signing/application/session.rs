//! Sesión local de firma trifásica: prefirma, firma en el token y postfirma (ADR-0001, ADR-0016).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::commands::Failure;
use crate::documents::adapters::views::SignedDocumentView;
use crate::documents::application::in_hand::DocumentInHand;
use crate::documents::application::opened::OpenedDocuments;
use crate::documents::application::{documents, recents};
use crate::documents::domain::portal::PortalDocument;
use crate::identity::adapters::pkcs11;
use crate::identity::application::certificates;
use crate::identity::application::certificates::StampedHolder;
use crate::identity::application::listed::ListedCertificates;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::secret::{SecretOnTheReaderKeypad, StoreSecret};
use crate::identity::domain::store::Store;
use crate::lock;
use crate::signing::adapters::isolate::Isolate;
use crate::signing::adapters::orders::SigningOrder;
use crate::signing::application::configuration_memory::Configuration;
use crate::signing::application::cycle::{
    self, CycleError, OpenCycle, SigningRequest, TokenSignature, NOTHING_FROM_A_SITE,
};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::{
    compose_layer2_text, AdmissibleDocument, SessionSeal, SignatureConfig, VisibleTextFields,
};
use crate::Memory;

/// Sesión de firma activa entre la prefirma y la postfirma (ADR-0016).
#[derive(Default)]
pub struct SigningSession {
    open: Mutex<Option<InFlight>>,
    delivered: Mutex<Option<PathBuf>>,
}

struct InFlight {
    cycle: OpenCycle,
    document: DocumentInHand,
    signature: Option<TokenSignature>,
    certificate: CertificateRef,
    signer_der: Vec<u8>,
    seal: SessionSeal,
}

/// Prefirma local: valida admisibilidad, prepara la configuración y abre el ciclo.
pub fn begin(
    order: &SigningOrder,
    stores: &[Store],
    listed: &ListedCertificates,
    opened: &OpenedDocuments,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, Failure> {
    let document = DocumentInHand::taken(opened, &order.document)?;
    let bytes = admitted_bytes(document.document())?;
    let (config, reference, chain) = plan_signature(stores, listed, order)?;
    Ok(open_the_cycle(
        document,
        bytes,
        config,
        reference,
        chain,
        &NOTHING_FROM_A_SITE,
        isolate,
        session,
    )?)
}

/// Errores tipados durante el tramo trifásico de firma.
#[derive(Debug)]
pub enum CycleFailure {
    /// El documento no se ha podido leer del disco.
    DocumentUnreadable(String),
    /// El ciclo ha fallado en alguna de sus comprobaciones.
    Cycle(CycleError),
    /// El secreto debe introducirse en el teclado del lector.
    SecretOnTheReaderKeypad(SecretOnTheReaderKeypad),
    /// El hilo del isolate no está disponible.
    Gone(IsolateGone),
}

impl From<CycleError> for CycleFailure {
    fn from(error: CycleError) -> Self {
        Self::Cycle(error)
    }
}

impl From<TokenError> for CycleFailure {
    fn from(error: TokenError) -> Self {
        Self::Cycle(CycleError::from(error))
    }
}

impl From<SecretOnTheReaderKeypad> for CycleFailure {
    fn from(refusal: SecretOnTheReaderKeypad) -> Self {
        Self::SecretOnTheReaderKeypad(refusal)
    }
}

impl From<IsolateGone> for CycleFailure {
    fn from(gone: IsolateGone) -> Self {
        Self::Gone(gone)
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "es el cuerpo compartido de dos casos de uso, no una interfaz"
)]
pub(crate) fn open_the_cycle(
    document: DocumentInHand,
    bytes: Vec<u8>,
    config: crate::signing::domain::SignatureConfig,
    reference: CertificateRef,
    chain: Vec<Vec<u8>>,
    from_the_site: &BTreeMap<String, String>,
    isolate: &Isolate,
    session: &SigningSession,
) -> Result<StoreSecret, CycleFailure> {
    let secret = pkcs11::store_secret(&reference)?.admitted()?;
    let certificate = reference.clone();
    let signer_der = chain.first().cloned().unwrap_or_default();
    let from_the_site = from_the_site.clone();

    let cycle = on_the_bridge_with_situation(isolate, move |bridge| {
        let document = AdmissibleDocument::check(&bytes)?;
        cycle::presign(
            bridge,
            SigningRequest {
                document,
                chain: &chain,
                config: &config,
                from_the_site: &from_the_site,
                certificate: &reference,
            },
        )
    })?;

    let seal = cycle.seal_in_transit();
    *lock(&session.open) = Some(InFlight {
        cycle,
        document,
        signature: None,
        certificate,
        signer_der,
        seal,
    });
    Ok(secret)
}

/// Fase de firma en el token PKCS#11 con el PIN proporcionado (ADR-0001).
pub fn sign_on_token(session: &SigningSession, pin: &str) -> Result<(), Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.as_mut().ok_or_else(no_open_cycle)?;
    in_flight.signature = Some(in_flight.cycle.sign_on_token(pin)?);
    Ok(())
}

/// Postfirma: verifica el sello, compone el PDF y lo entrega en destino (ADR-0011, ADR-0016).
pub fn finish(
    isolate: &Isolate,
    session: &SigningSession,
    memory: &Memory,
    configuration: &Configuration,
    documents_folder: &Path,
) -> Result<SignedDocumentView, Failure> {
    let SignedCycle {
        cycle,
        document,
        signature,
        seal,
        certificate,
        ..
    } = take_signed_cycle(session)?;

    let signed = on_the_bridge(isolate, move |bridge| {
        cycle.postsign(bridge, &signature, &seal)
    })?;

    let (landing, delivered) = documents::deliver(
        configuration,
        documents_folder,
        document.document(),
        &signed,
    )?;
    certificates::remember_the_certificate(memory, configuration, &certificate);
    if document.is_remembered() {
        recents::note_signed(memory, configuration, &landing);
    }
    *lock(&session.delivered) = Some(landing);
    Ok(delivered)
}

/// Ruta del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_document(session: &SigningSession) -> Result<PathBuf, Failure> {
    lock(&session.delivered)
        .clone()
        .ok_or_else(no_signed_document)
}

/// Directorio del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_folder(session: &SigningSession) -> Result<PathBuf, Failure> {
    let landing = signed_document(session)?;
    landing
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(no_signed_document)
}

fn no_signed_document() -> Failure {
    Failure::new("unknown", "no hay ningun documento firmado en esta sesion")
}

/// Indica si hay una sesión de firma activa en curso.
pub fn is_live(session: &SigningSession) -> bool {
    lock(&session.open).is_some()
}

/// Cancela la sesión activa descartando el ciclo en curso.
pub fn cancel(session: &SigningSession) {
    *lock(&session.open) = None;
}

fn layer2_text_of(order: &SigningOrder, holder: &StampedHolder) -> String {
    compose_layer2_text(
        &VisibleTextFields {
            signer_name: order
                .fields
                .signer_name
                .then_some(holder.common_name.as_str())
                .filter(|name| !name.is_empty()),
            issuer: order
                .fields
                .issuer
                .then_some(holder.issuer.as_str())
                .filter(|issuer| !issuer.is_empty()),
            signed_at: order.fields.signed_at.then_some(order.signed_at.as_str()),
            reason: order
                .fields
                .reason
                .then_some(order.reason.as_str())
                .filter(|reason| !reason.is_empty()),
            pseudonym: holder.pseudonym,
        },
        super::configuration::language_of(&order.language),
    )
}

/// Configuración de firma construida a partir de la orden y del certificado seleccionado.
pub fn config_for(
    order: &SigningOrder,
    chosen: &TokenCertificate,
) -> Result<SignatureConfig, Failure> {
    let holder = certificates::stamped_holder_of(chosen);
    Ok(SignatureConfig {
        placement: order
            .placement
            .as_ref()
            .map(|placement| placement.placement())
            .transpose()?,
        layer2_text: layer2_text_of(order, &holder),
        rubric_image: order.rubric.clone(),
        sign_reason: (!order.reason.is_empty()).then(|| order.reason.clone()),
        allow_unregistered_signatures: order.allow_unregistered_signatures,
    })
}

pub(crate) fn plan_signature(
    stores: &[Store],
    listed: &ListedCertificates,
    order: &SigningOrder,
) -> Result<(SignatureConfig, CertificateRef, Vec<Vec<u8>>), Failure> {
    let found = pkcs11::list_certificates_across(stores)?;
    let chosen = certificates::usable_certificate(&found, &order.certificate, listed)?;
    Ok((
        config_for(order, chosen)?,
        chosen.reference().clone(),
        vec![chosen.der().to_vec()],
    ))
}

/// Obtiene y valida los bytes de un documento para firmar.
pub fn admitted_bytes(document: &PortalDocument) -> Result<Vec<u8>, Failure> {
    admitted_bytes_with_situation(document).map_err(Failure::from)
}

pub(crate) fn admitted_bytes_with_situation(
    document: &PortalDocument,
) -> Result<Vec<u8>, CycleFailure> {
    let bytes = std::fs::read(document.reading_path())
        .map_err(|error| CycleFailure::DocumentUnreadable(error.to_string()))?;
    AdmissibleDocument::check(&bytes).map_err(CycleError::from)?;
    Ok(bytes)
}

/// Comprueba si el documento contiene firmas previas no reconocibles.
pub fn unregistered_signatures_in(
    opened: &OpenedDocuments,
    document: &str,
) -> Result<bool, Failure> {
    let in_hand = DocumentInHand::taken(opened, document)?;
    let bytes = admitted_bytes(in_hand.document())?;
    Ok(AdmissibleDocument::check(&bytes)?.has_unregistered_signatures())
}

/// Extrae el ciclo completado en el token de la sesión activa.
pub fn take_signed_cycle(session: &SigningSession) -> Result<SignedCycle, Failure> {
    let mut open = lock(&session.open);
    let in_flight = open.take().ok_or_else(no_open_cycle)?;
    let signature = in_flight
        .signature
        .ok_or_else(|| Failure::new("unknown", "todavía no se ha firmado en el token"))?;
    Ok(SignedCycle {
        cycle: in_flight.cycle,
        document: in_flight.document,
        signature,
        seal: in_flight.seal,
        certificate: in_flight.certificate,
        signer_der: in_flight.signer_der,
    })
}

/// Ciclo firmado en el token preparado para la postfirma.
pub struct SignedCycle {
    pub cycle: OpenCycle,
    pub document: DocumentInHand,
    pub signature: TokenSignature,
    pub seal: SessionSeal,
    pub certificate: CertificateRef,
    pub signer_der: Vec<u8>,
}

pub(crate) fn on_the_bridge<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::signing::adapters::ffi::NativeBridge) -> Result<T, cycle::CycleError>
        + Send
        + 'static,
) -> Result<T, Failure> {
    on_the_bridge_with_situation(isolate, task).map_err(Failure::from)
}

pub(crate) fn on_the_bridge_with_situation<T: Send + 'static>(
    isolate: &Isolate,
    task: impl FnOnce(&crate::signing::adapters::ffi::NativeBridge) -> Result<T, cycle::CycleError>
        + Send
        + 'static,
) -> Result<T, CycleFailure> {
    match isolate.run(task) {
        Err(gone) => Err(gone.into()),
        Ok(Err(bridge)) => Err(CycleError::from(bridge).into()),
        Ok(Ok(outcome)) => outcome.map_err(CycleFailure::from),
    }
}

fn no_open_cycle() -> Failure {
    Failure::new("unknown", "no hay ninguna firma empezada")
}

#[cfg(test)]
mod tests;
