//! Estado del trámite con la sede y gestión de su ciclo de vida (ADR-0016).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::protocol::{AfirmaUrl, ChannelCredential, SiteFilter};

use super::outcome::{Moment, SiteOutcome};
use super::ports::{ProtocolCodec, ReplyHandle};

/// Códec negociado, compartido entre el trámite y quien lo apuntó.
pub type NegotiatedCodec = Arc<dyn ProtocolCodec + Send + Sync>;

/// Trámite vivo del proceso y su memoria durante la ejecución.
#[derive(Default)]
pub struct LiveErrand {
    errand: Mutex<Option<Errand>>,
    codec: Mutex<Option<NegotiatedCodec>>,
    scratch: Mutex<Option<PathBuf>>,
    reply: Mutex<Option<ReplyHandle>>,
    asked: Mutex<Option<AfirmaUrl>>,
    consent: Mutex<Option<PendingConsent>>,
    moment: Mutex<Option<Moment>>,
}

/// Datos identificativos y de conexión de un trámite en curso.
#[derive(Clone)]
pub struct Errand {
    credential: ChannelCredential,
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
    pub fn of(credential: ChannelCredential, port: u16, codec: NegotiatedCodec) -> Self {
        Self {
            credential,
            port,
            codec,
        }
    }

    /// Credencial con la que se cerró el canal.
    pub fn credential(&self) -> &ChannelCredential {
        &self.credential
    }

    /// Puerto en el que quedó escuchando el servidor.
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Consentimiento pendiente según la operación solicitada.
enum PendingConsent {
    Identity(SiteFilter),
    Signature(PendingSignature),
}

/// Datos necesarios para ejecutar la firma tras el consentimiento.
#[derive(Clone, Debug)]
pub(super) struct PendingSignature {
    /// Identificador del documento para la ventana.
    pub(super) document: String,
    /// Filtro solicitado por la sede.
    pub(super) filter: SiteFilter,
    /// Parámetros adicionales expandidos.
    pub(super) from_the_site: BTreeMap<String, String>,
    /// Si el documento contiene firmas no reconocidas.
    pub(super) unregistered_signatures: bool,
}

impl LiveErrand {
    /// Trámite de prueba inicializado con un códec específico.
    #[cfg(test)]
    pub fn speaking(codec: NegotiatedCodec) -> Self {
        let live = Self::default();
        *crate::app::lock(&live.codec) = Some(codec);
        live
    }

    /// Registra la ruta temporal del documento de paso.
    pub(super) fn keep_the_scratch(&self, path: PathBuf) {
        *crate::app::lock(&self.scratch) = Some(path);
    }

    /// Registra la petición original de la sede.
    pub(super) fn keep_the_request(&self, url: AfirmaUrl) {
        *crate::app::lock(&self.asked) = Some(url);
    }

    /// Registra el inicio de un trámite si no hay otro activo.
    #[must_use = "con uno vivo devuelve false y el que llega no queda apuntado"]
    pub fn begin(&self, errand: Errand) -> bool {
        let mut live = crate::app::lock(&self.errand);
        if live.is_some() {
            return false;
        }
        *crate::app::lock(&self.codec) = Some(Arc::clone(&errand.codec));
        *live = Some(errand);
        true
    }

    /// Registra el asa de respuesta para contestar a la sede.
    pub fn answer_through(&self, reply: ReplyHandle) {
        *crate::app::lock(&self.reply) = Some(reply);
    }

    /// Envía la respuesta codificada a la sede a través del asa.
    pub(super) fn answer_the_site(&self, outcome: &SiteOutcome) {
        let Some(reply) = crate::app::lock(&self.reply).take() else {
            return;
        };
        if let Some(codec) = self.codec() {
            reply.answer(codec.encode(outcome));
        }
    }

    /// Códec negociado para el canal de este trámite.
    pub fn codec(&self) -> Option<NegotiatedCodec> {
        crate::app::lock(&self.codec).clone()
    }

    /// Petición original recibida de la sede.
    pub fn the_request(&self) -> Option<AfirmaUrl> {
        crate::app::lock(&self.asked).clone()
    }

    /// Trámite activo actual, si lo hay.
    pub fn current(&self) -> Option<Errand> {
        crate::app::lock(&self.errand).clone()
    }

    /// Finaliza el trámite y limpia sus recursos asociados.
    pub fn end(&self) {
        *crate::app::lock(&self.errand) = None;
        drop(crate::app::lock(&self.reply).take());
        if let Some(scratch) = crate::app::lock(&self.scratch).take() {
            let _ = std::fs::remove_file(scratch);
        }
        *crate::app::lock(&self.asked) = None;
        self.forget_the_consent();
    }

    /// Ruta al fichero temporal para pruebas.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        crate::app::lock(&self.scratch).clone()
    }

    /// Registra el filtro de consentimiento de identidad.
    pub(super) fn remember_identity(&self, filter: SiteFilter) {
        *crate::app::lock(&self.consent) = Some(PendingConsent::Identity(filter));
    }

    /// Registra los datos de consentimiento de firma.
    pub(super) fn remember_signature(&self, pending: PendingSignature) {
        *crate::app::lock(&self.consent) = Some(PendingConsent::Signature(pending));
    }

    /// Filtro de identidad pendiente, si lo hay.
    pub(super) fn what_the_site_asked(&self) -> Option<SiteFilter> {
        match &*crate::app::lock(&self.consent) {
            Some(PendingConsent::Identity(filter)) => Some(filter.clone()),
            _ => None,
        }
    }

    /// Firma consentida pendiente, si la hay.
    pub(super) fn the_signature_consented(&self) -> Option<PendingSignature> {
        match &*crate::app::lock(&self.consent) {
            Some(PendingConsent::Signature(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Limpia los datos de consentimiento registrados.
    pub(super) fn forget_the_consent(&self) {
        *crate::app::lock(&self.consent) = None;
    }

    /// Registra el momento actual del trámite.
    pub fn note(&self, moment: Moment) {
        *crate::app::lock(&self.moment) = Some(moment);
    }

    /// Último momento registrado del trámite.
    pub fn moment(&self) -> Option<Moment> {
        crate::app::lock(&self.moment).clone()
    }
}

#[cfg(test)]
mod tests;
