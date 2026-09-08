//! Estado del trámite con la sede y gestión de su ciclo de vida (ADR-0016).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::{Format, SignatureOperation};
use crate::site::domain::protocol::{AfirmaUrl, BatchRequest, NegotiatedCredential, SiteFilter};

use super::outcome::{
    LoadingConsent, Moment, ProtocolCodec, SavingConsent, SavingHints, SiteOutcome,
};
use crate::site::ports::{ReplyHandle, Scratch};

/// Códec negociado, compartido entre el trámite y quien lo apuntó.
pub type NegotiatedCodec = Arc<dyn ProtocolCodec + Send + Sync>;

struct KeptScratch {
    path: PathBuf,
    files: Arc<dyn Scratch + Send + Sync>,
}

/// Trámite vivo del proceso y su memoria durante la ejecución.
#[derive(Default)]
pub struct LiveErrand {
    errand: Mutex<Option<Errand>>,
    codec: Mutex<Option<NegotiatedCodec>>,
    scratch: Mutex<Option<KeptScratch>>,
    reply: Mutex<Option<ReplyHandle>>,
    asked: Mutex<Option<AfirmaUrl>>,
    consent: Mutex<Option<PendingConsent>>,
    moment: Mutex<Option<Moment>>,
}

/// Datos identificativos y de conexión de un trámite en curso.
#[derive(Clone)]
pub struct Errand {
    credential: NegotiatedCredential,
    port: u16,
    codec: NegotiatedCodec,
}

impl std::fmt::Debug for Errand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Errand")
            .field("credential", &self.credential)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

impl Errand {
    /// Construye un trámite con la credencial, puerto y códec indicados.
    pub fn of(credential: NegotiatedCredential, port: u16, codec: NegotiatedCodec) -> Self {
        Self {
            credential,
            port,
            codec,
        }
    }

    /// Credencial con la que se cerró el canal, si la sede la exigió.
    pub fn credential(&self) -> &NegotiatedCredential {
        &self.credential
    }

    /// Puerto en el que quedó escuchando el servidor.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Códec negociado para este trámite.
    pub fn codec(&self) -> &NegotiatedCodec {
        &self.codec
    }
}

/// Consentimiento pendiente según la operación solicitada.
enum PendingConsent {
    Identity(SiteFilter, bool),
    Signature(PendingSignature),
    Batch(PendingBatch),
    Saving(SavingConsent),
    Loading(LoadingConsent),
}

/// Lo que el lote remoto necesita entre el consentimiento y la postfirma.
#[derive(Clone, Debug)]
pub(super) struct PendingBatch {
    /// El lote tal y como lo pidió la sede.
    pub(super) request: BatchRequest,
    /// El certificado que la persona consintió, una vez consentido.
    pub(super) chosen: Option<TokenCertificate>,
}

/// Datos necesarios para ejecutar la firma tras el consentimiento.
#[derive(Clone, Debug)]
pub(super) struct PendingSignature {
    /// Identificador del documento para la ventana.
    pub(super) document: String,
    /// Filtro solicitado por la sede.
    pub(super) filter: SiteFilter,
    /// Formato de firma que pidió la sede, ya atendido por el puente.
    pub(super) format: Format,
    /// Qué pidió hacer la sede con el documento.
    pub(super) operation: SignatureOperation,
    /// Parámetros adicionales expandidos.
    pub(super) from_the_site: BTreeMap<String, String>,
    /// Si el documento contiene firmas no reconocidas.
    pub(super) unregistered_signatures: bool,
    /// Pistas de guardado, si esta firma viene de `signandsave`.
    pub(super) saving: Option<Box<SavingHints>>,
}

impl LiveErrand {
    /// Trámite de prueba inicializado con un códec específico.
    #[cfg(test)]
    pub fn speaking(codec: NegotiatedCodec) -> Self {
        let live = Self::default();
        *crate::lock(&live.codec) = Some(codec);
        live
    }

    /// Registra la ruta temporal del documento de paso.
    pub(super) fn keep_the_scratch(&self, path: PathBuf, files: Arc<dyn Scratch + Send + Sync>) {
        *crate::lock(&self.scratch) = Some(KeptScratch { path, files });
    }

    /// Registra la petición original de la sede.
    pub(super) fn keep_the_request(&self, url: AfirmaUrl) {
        *crate::lock(&self.asked) = Some(url);
    }

    /// Registra el inicio de un trámite si no hay otro activo.
    #[must_use = "con uno vivo devuelve false y el que llega no queda apuntado"]
    pub fn begin(&self, errand: Errand) -> bool {
        let mut live = crate::lock(&self.errand);
        if live.is_some() {
            return false;
        }
        *crate::lock(&self.codec) = Some(Arc::clone(&errand.codec));
        *live = Some(errand);
        true
    }

    /// Registra el asa de respuesta para contestar a la sede.
    pub fn answer_through(&self, reply: ReplyHandle) {
        *crate::lock(&self.reply) = Some(reply);
    }

    /// Envía la respuesta codificada a la sede a través del asa.
    pub(super) fn answer_the_site(&self, outcome: &SiteOutcome) {
        let Some(reply) = crate::lock(&self.reply).take() else {
            return;
        };
        if let Some(codec) = self.codec() {
            reply.answer(codec.encode(outcome));
        }
    }

    /// Códec negociado para el canal de este trámite.
    pub fn codec(&self) -> Option<NegotiatedCodec> {
        crate::lock(&self.codec).clone()
    }

    /// Petición original recibida de la sede.
    pub fn the_request(&self) -> Option<AfirmaUrl> {
        crate::lock(&self.asked).clone()
    }

    /// Trámite activo actual, si lo hay.
    pub fn current(&self) -> Option<Errand> {
        crate::lock(&self.errand).clone()
    }

    /// Finaliza el trámite y limpia sus recursos asociados.
    pub fn end(&self) {
        *crate::lock(&self.errand) = None;
        drop(crate::lock(&self.reply).take());
        if let Some(scratch) = crate::lock(&self.scratch).take() {
            scratch.files.erase(&scratch.path);
        }
        *crate::lock(&self.asked) = None;
        self.forget_the_consent();
    }

    /// Ruta al fichero temporal para pruebas.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        crate::lock(&self.scratch)
            .as_ref()
            .map(|scratch| scratch.path.clone())
    }

    /// Registra el filtro de consentimiento de identidad y si la sede lo pegó.
    pub(super) fn remember_identity(&self, filter: SiteFilter, sticky: bool) {
        *crate::lock(&self.consent) = Some(PendingConsent::Identity(filter, sticky));
    }

    /// Registra los datos de consentimiento de firma.
    pub(super) fn remember_signature(&self, pending: PendingSignature) {
        *crate::lock(&self.consent) = Some(PendingConsent::Signature(pending));
    }

    /// Registra el lote pendiente de consentimiento o de postfirma.
    pub(super) fn remember_the_batch(&self, pending: PendingBatch) {
        *crate::lock(&self.consent) = Some(PendingConsent::Batch(pending));
    }

    /// Si el trámite tiene un lote consentido esperando el secreto.
    pub fn a_batch_is_pending(&self) -> bool {
        matches!(
            &*crate::lock(&self.consent),
            Some(PendingConsent::Batch(pending)) if pending.chosen.is_some()
        )
    }

    /// Lote pendiente, si el trámite está atendiendo uno.
    pub(super) fn the_batch_pending(&self) -> Option<PendingBatch> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Batch(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Registra los datos del diálogo de guardado pendiente.
    pub(super) fn remember_saving(&self, consent: SavingConsent) {
        *crate::lock(&self.consent) = Some(PendingConsent::Saving(consent));
    }

    /// Registra los datos del selector de carga pendiente.
    pub(super) fn remember_loading(&self, consent: LoadingConsent) {
        *crate::lock(&self.consent) = Some(PendingConsent::Loading(consent));
    }

    /// Datos del diálogo de guardado pendiente, si el trámite está esperando uno.
    pub fn the_saving_pending(&self) -> Option<SavingConsent> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Saving(consent)) => Some(consent.clone()),
            _ => None,
        }
    }

    /// Datos del selector de carga pendiente, si el trámite está esperando uno.
    pub fn the_loading_pending(&self) -> Option<LoadingConsent> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Loading(consent)) => Some(consent.clone()),
            _ => None,
        }
    }

    /// Filtro de identidad pendiente y si la sede lo pegó, si lo hay.
    pub(super) fn what_the_site_asked(&self) -> Option<(SiteFilter, bool)> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Identity(filter, sticky)) => Some((filter.clone(), *sticky)),
            _ => None,
        }
    }

    /// Firma consentida pendiente, si la hay.
    pub(super) fn the_signature_consented(&self) -> Option<PendingSignature> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Signature(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Limpia los datos de consentimiento registrados.
    pub(super) fn forget_the_consent(&self) {
        *crate::lock(&self.consent) = None;
    }

    /// Registra el momento actual del trámite.
    pub fn note(&self, moment: Moment) {
        *crate::lock(&self.moment) = Some(moment);
    }

    /// Último momento registrado del trámite.
    pub fn moment(&self) -> Option<Moment> {
        crate::lock(&self.moment).clone()
    }
}

#[cfg(test)]
mod tests;
