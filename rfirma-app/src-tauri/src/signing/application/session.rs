//! Sesión local de firma trifásica: prefirma, firma en el token y postfirma (ADR-0001, ADR-0016).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::documents::domain::document::Document;
use crate::documents::domain::error::DocumentError;
use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::identity::domain::error::TokenError;
use crate::identity::domain::holder::{stamped_holder_of, StampedHolder};
use crate::identity::domain::secret::{SecretOnTheReaderKeypad, StoreSecret};
use crate::lock;
use crate::signing::application::cycle::{
    self, CycleError, OpenCycle, SigningRequest, NOTHING_FROM_A_SITE,
};
use crate::signing::domain::isolate_gone::IsolateGone;
use crate::signing::domain::{
    compose_layer2_text, AdmissibleDocument, CompletedCycle, Format, PlacementError, SessionSeal,
    SignatureConfig, SigningChoice, VisibleTextFields,
};
use crate::signing::domain::{Refusal, SignatureOperation, TokenSignatures};
use crate::signing::ports::{DocumentBytes, IsolateHost, Signer};

/// Sesión de firma activa entre la prefirma y la postfirma (ADR-0016).
#[derive(Default)]
pub struct SigningSession {
    open: Mutex<Option<InFlight>>,
    delivered: Mutex<Option<PathBuf>>,
}

struct InFlight {
    cycle: OpenCycle,
    handle: String,
    document: Document,
    signature: Option<TokenSignatures>,
    certificate: CertificateRef,
    signer_der: Vec<u8>,
    seal: SessionSeal,
}

/// El documento a firmar: el asa con la que lo nombra la ventana y lo que hay detrás.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentToSign {
    /// El asa que dio el portal al abrirlo.
    pub handle: String,
    /// El documento tal como entró por el portal.
    pub document: Document,
}

/// Prefirma local: valida admisibilidad, prepara la configuración y abre el ciclo.
pub fn begin(
    files: &dyn DocumentBytes,
    document: DocumentToSign,
    chosen: &TokenCertificate,
    choice: &SigningChoice,
    signer: &dyn Signer,
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<StoreSecret, CycleFailure> {
    let bytes = admitted_bytes(files, &document.document, Format::Pades)?;
    let config = config_for(choice, chosen)?;
    open_the_cycle(
        signer,
        Format::Pades,
        SignatureOperation::Sign,
        document,
        bytes,
        config,
        chosen,
        &NOTHING_FROM_A_SITE,
        isolate,
        session,
    )
}

/// Lo que la sede declaró para esta firma: sus parámetros y si consintió cofirmar sobre lo que no se reconoce.
#[derive(Clone, Copy, Debug)]
pub struct DeclaredByTheSite<'a> {
    /// El formato de firma que pidió la sede.
    pub format: Format,
    /// Qué pidió hacer la sede con el documento: firmarlo, cofirmarlo o contrafirmarlo.
    pub operation: SignatureOperation,
    /// Los parámetros de la sede, ya expandidos.
    pub parameters: &'a BTreeMap<String, String>,
    /// Si la sede consintió cofirmar sobre firmas que no se reconocen.
    pub allow_unregistered_signatures: bool,
}

/// Prefirma de un trámite de sede: invisible, con la geometría y la política que la sede declaró.
pub fn begin_for_the_site(
    files: &dyn DocumentBytes,
    document: DocumentToSign,
    chosen: &TokenCertificate,
    declared: DeclaredByTheSite<'_>,
    signer: &dyn Signer,
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<StoreSecret, CycleFailure> {
    let bytes = admitted_bytes(files, &document.document, declared.format)?;
    let config = config_for(
        &SigningChoice::for_the_site(declared.allow_unregistered_signatures),
        chosen,
    )?;
    open_the_cycle(
        signer,
        declared.format,
        declared.operation,
        document,
        bytes,
        config,
        chosen,
        declared.parameters,
        isolate,
        session,
    )
}

/// Por qué la firma local no ha salido, desde abrir el documento hasta entregarlo.
#[derive(Debug)]
pub enum CycleFailure {
    /// El documento no se ha podido abrir, leer ni entregar.
    Document(DocumentError),
    /// La colocación del recuadro no vale.
    Placement(PlacementError),
    /// El ciclo ha fallado en alguna de sus comprobaciones.
    Cycle(CycleError),
    /// El secreto debe introducirse en el teclado del lector.
    SecretOnTheReaderKeypad(SecretOnTheReaderKeypad),
    /// El hilo del isolate no está disponible.
    Gone(IsolateGone),
    /// No hay ninguna firma empezada.
    NoOpenCycle,
    /// Todavía no se ha firmado en el token.
    NotSignedYet,
    /// No hay ningún documento firmado en esta sesión.
    NoSignedDocument,
}

impl From<DocumentError> for CycleFailure {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}

impl From<PlacementError> for CycleFailure {
    fn from(error: PlacementError) -> Self {
        Self::Placement(error)
    }
}

impl From<Refusal> for CycleFailure {
    fn from(refusal: Refusal) -> Self {
        Self::Cycle(CycleError::from(refusal))
    }
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
fn open_the_cycle(
    signer: &dyn Signer,
    format: Format,
    operation: SignatureOperation,
    document: DocumentToSign,
    bytes: Vec<u8>,
    config: SignatureConfig,
    chosen: &TokenCertificate,
    from_the_site: &BTreeMap<String, String>,
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<StoreSecret, CycleFailure> {
    let reference = chosen.reference().clone();
    let secret = signer.secret_of(&reference)?.admitted()?;
    let certificate = reference.clone();
    let signer_der = chosen.der().to_vec();
    let chain = vec![signer_der.clone()];
    let from_the_site = from_the_site.clone();

    let cycle = on_the_bridge(isolate, move |bridge| {
        let document = AdmissibleDocument::check_for(format, &bytes)?;
        cycle::presign(
            bridge,
            SigningRequest {
                format,
                operation,
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
        handle: document.handle,
        document: document.document,
        signature: None,
        certificate,
        signer_der,
        seal,
    });
    Ok(secret)
}

/// Fase de firma en el token PKCS#11 con el PIN proporcionado (ADR-0001).
pub fn sign_on_token(
    signer: &dyn Signer,
    session: &SigningSession,
    pin: &str,
) -> Result<(), CycleFailure> {
    let mut open = lock(&session.open);
    let in_flight = open.as_mut().ok_or(CycleFailure::NoOpenCycle)?;
    in_flight.signature = Some(in_flight.cycle.sign_on_token(signer, pin)?);
    Ok(())
}

/// Lo que sale de la postfirma: el ciclo completado y con qué documento y certificado se hizo.
pub struct Signed {
    /// El asa con la que la ventana nombra el documento firmado.
    pub handle: String,
    /// El documento que se firmó.
    pub document: Document,
    /// El ciclo completado, con el PDF firmado dentro.
    pub completed: CompletedCycle,
    /// El certificado con el que se firmó.
    pub certificate: CertificateRef,
    /// El DER del firmante.
    pub signer_der: Vec<u8>,
}

/// Postfirma: verifica el sello y compone el PDF; entregarlo o no es de quien llama (ADR-0011, ADR-0016).
pub fn finish(
    isolate: &impl IsolateHost,
    session: &SigningSession,
) -> Result<Signed, CycleFailure> {
    let SignedCycle {
        cycle,
        handle,
        document,
        signature,
        seal,
        certificate,
        signer_der,
    } = take_signed_cycle(session)?;

    let completed = on_the_bridge(isolate, move |bridge| {
        cycle.postsign(bridge, signature, &seal)
    })?;

    Ok(Signed {
        handle,
        document,
        completed,
        certificate,
        signer_der,
    })
}

/// Apunta dónde quedó el último documento firmado de esta sesión (ADR-0011).
pub fn note_delivered(session: &SigningSession, landing: PathBuf) {
    *lock(&session.delivered) = Some(landing);
}

/// Ruta del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_document(session: &SigningSession) -> Result<PathBuf, CycleFailure> {
    lock(&session.delivered)
        .clone()
        .ok_or(CycleFailure::NoSignedDocument)
}

/// Directorio del último documento firmado entregado en esta sesión (ADR-0011).
pub fn signed_folder(session: &SigningSession) -> Result<PathBuf, CycleFailure> {
    let landing = signed_document(session)?;
    landing
        .parent()
        .map(Path::to_path_buf)
        .ok_or(CycleFailure::NoSignedDocument)
}

/// Indica si hay una sesión de firma activa en curso.
pub fn is_live(session: &SigningSession) -> bool {
    lock(&session.open).is_some()
}

/// Cancela la sesión activa descartando el ciclo en curso.
pub fn cancel(session: &SigningSession) {
    *lock(&session.open) = None;
}

fn layer2_text_of(choice: &SigningChoice, holder: &StampedHolder) -> String {
    compose_layer2_text(
        &VisibleTextFields {
            signer_name: choice
                .fields
                .signer_name
                .then_some(holder.common_name.as_str())
                .filter(|name| !name.is_empty()),
            issuer: choice
                .fields
                .issuer
                .then_some(holder.issuer.as_str())
                .filter(|issuer| !issuer.is_empty()),
            signed_at: choice.fields.signed_at.then_some(choice.signed_at.as_str()),
            reason: choice
                .fields
                .reason
                .then_some(choice.reason.as_str())
                .filter(|reason| !reason.is_empty()),
            pseudonym: holder.pseudonym,
        },
        choice.language,
    )
}

/// Configuración de firma construida a partir de lo elegido y del certificado seleccionado.
pub fn config_for(
    choice: &SigningChoice,
    chosen: &TokenCertificate,
) -> Result<SignatureConfig, PlacementError> {
    let holder = stamped_holder_of(chosen);
    Ok(SignatureConfig {
        placement: choice.placement.clone(),
        layer2_text: layer2_text_of(choice, &holder),
        rubric_image: choice.rubric.clone(),
        sign_reason: (!choice.reason.is_empty()).then(|| choice.reason.clone()),
        allow_unregistered_signatures: choice.allow_unregistered_signatures,
    })
}

/// Obtiene y valida los bytes de un documento para firmar.
pub fn admitted_bytes(
    files: &dyn DocumentBytes,
    document: &Document,
    format: Format,
) -> Result<Vec<u8>, CycleFailure> {
    let bytes = files
        .read(document.reading_path())
        .map_err(DocumentError::Unreadable)?;
    AdmissibleDocument::check_for(format, &bytes).map_err(CycleError::from)?;
    Ok(bytes)
}

/// Comprueba si el documento contiene firmas previas no reconocibles.
pub fn unregistered_signatures_in(
    files: &dyn DocumentBytes,
    document: &Document,
) -> Result<bool, CycleFailure> {
    let bytes = admitted_bytes(files, document, Format::Pades)?;
    Ok(AdmissibleDocument::check(&bytes)?.has_unregistered_signatures())
}

/// Extrae el ciclo completado en el token de la sesión activa.
pub fn take_signed_cycle(session: &SigningSession) -> Result<SignedCycle, CycleFailure> {
    let mut open = lock(&session.open);
    let in_flight = open.take().ok_or(CycleFailure::NoOpenCycle)?;
    let signature = in_flight.signature.ok_or(CycleFailure::NotSignedYet)?;
    Ok(SignedCycle {
        cycle: in_flight.cycle,
        handle: in_flight.handle,
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
    pub handle: String,
    pub document: Document,
    pub signature: TokenSignatures,
    pub seal: SessionSeal,
    pub certificate: CertificateRef,
    pub signer_der: Vec<u8>,
}

pub(crate) fn on_the_bridge<T: Send + 'static>(
    isolate: &impl IsolateHost,
    task: impl FnOnce(&dyn crate::signing::ports::Bridge) -> Result<T, cycle::CycleError>
        + Send
        + 'static,
) -> Result<T, CycleFailure> {
    match isolate.run(task) {
        Err(gone) => Err(gone.into()),
        Ok(Err(bridge)) => Err(CycleError::from(bridge).into()),
        Ok(Ok(outcome)) => outcome.map_err(CycleFailure::from),
    }
}

#[cfg(test)]
mod tests;
