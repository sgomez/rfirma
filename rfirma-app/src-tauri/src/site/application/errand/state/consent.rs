//! El consentimiento pendiente entre la petición de la sede y la respuesta que lo resuelve.

use std::collections::BTreeMap;

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::{Format, SignatureOperation};
use crate::site::domain::batch::LocalBatch;
use crate::site::domain::protocol::{
    AskedAlgorithm, BatchRequest, Refusal, SignatureRound, SiteFilter,
};
use crate::site::domain::signing::SiteSignature;
use crate::site::domain::triphase_server::ServerFormat;

use crate::site::application::errand::outcome::{
    ConfirmationConsent, LoadingConsent, SavingConsent, SavingHints,
};

use super::{AreaToMark, LiveErrand};

/// Consentimiento pendiente según la operación solicitada.
pub(super) enum PendingConsent {
    Identity(SiteFilter, bool),
    Signature(PendingSignature),
    Confirmation(ConfirmationConsent),
    Batch(PendingBatch),
    LocalBatch(PendingLocalBatch),
    Saving(SavingConsent),
    Loading(LoadingConsent),
    ShownRefusal(Refusal),
}

/// Lo que el lote remoto necesita entre el consentimiento y la postfirma.
#[derive(Clone, Debug)]
pub(in crate::site::application::errand) struct PendingBatch {
    /// El lote tal y como lo pidió la sede.
    pub(in crate::site::application::errand) request: BatchRequest,
    /// El certificado que la persona consintió, una vez consentido.
    pub(in crate::site::application::errand) chosen: Option<TokenCertificate>,
}

/// Lo que el lote local necesita entre el consentimiento y el bucle de firma.
#[derive(Clone, Debug)]
pub(in crate::site::application::errand) struct PendingLocalBatch {
    /// El lote tal y como lo pidió la sede.
    pub(in crate::site::application::errand) request: BatchRequest,
    /// Las firmas del lote, o por qué no se pudieron leer.
    pub(in crate::site::application::errand) batch: Result<LocalBatch, Refusal>,
    /// El certificado que la persona consintió, una vez consentido.
    pub(in crate::site::application::errand) chosen: Option<TokenCertificate>,
}

/// Datos necesarios para ejecutar la firma tras el consentimiento.
#[derive(Clone, Debug)]
pub(in crate::site::application::errand) struct PendingSignature {
    /// Identificador del documento para la ventana.
    pub(in crate::site::application::errand) document: String,
    /// Filtro solicitado por la sede.
    pub(in crate::site::application::errand) filter: SiteFilter,
    /// Formato de firma que pidió la sede, ya atendido por el puente.
    pub(in crate::site::application::errand) format: Format,
    /// Huella que pidió la sede para esta firma.
    pub(in crate::site::application::errand) algorithm: AskedAlgorithm,
    /// Qué pidió hacer la sede con el documento.
    pub(in crate::site::application::errand) operation: SignatureOperation,
    /// Parámetros adicionales expandidos.
    pub(in crate::site::application::errand) from_the_site: BTreeMap<String, String>,
    /// Si el documento contiene firmas no reconocidas.
    pub(in crate::site::application::errand) unregistered_signatures: bool,
    /// Si la sede pidió `headless`: lo que haga falta preguntar se rechaza.
    pub(in crate::site::application::errand) headless: bool,
    /// Pistas de guardado, si esta firma viene de `signandsave`.
    pub(in crate::site::application::errand) saving: Option<Box<SavingHints>>,
    /// La firma que hace el servidor trifásico de la sede, si se hace allí.
    pub(in crate::site::application::errand) through_the_server: Option<ServerSignature>,
    /// El área de la firma visible que falta por marcar, si falta.
    pub(in crate::site::application::errand) area: Option<AreaToMark>,
}

/// Lo que la firma contra el servidor trifásico lleva del consentimiento a la entrega.
#[derive(Clone, Debug)]
pub(in crate::site::application::errand) struct ServerSignature {
    /// El firmador trifásico que eligió la sede.
    pub(in crate::site::application::errand) format: ServerFormat,
    /// Los datos, o la firma previa en cofirma y contrafirma.
    pub(in crate::site::application::errand) document: Vec<u8>,
    /// La operación que pidió la sede.
    pub(in crate::site::application::errand) round: SignatureRound,
    /// El certificado que la persona consintió, una vez consentido.
    pub(in crate::site::application::errand) chosen: Option<TokenCertificate>,
    /// La firma que devolvió la postfirma, una vez hecha.
    pub(in crate::site::application::errand) signed: Option<SiteSignature>,
}

impl LiveErrand {
    /// Registra el filtro de consentimiento de identidad y si la sede lo pegó.
    pub(in crate::site::application::errand) fn remember_identity(
        &self,
        filter: SiteFilter,
        sticky: bool,
    ) {
        *crate::lock(&self.consent) = Some(PendingConsent::Identity(filter, sticky));
    }

    /// Registra los datos de consentimiento de firma.
    pub(in crate::site::application::errand) fn remember_signature(
        &self,
        pending: PendingSignature,
    ) {
        *crate::lock(&self.consent) = Some(PendingConsent::Signature(pending));
    }

    /// Registra la confirmación pendiente de una firma que el validador no da por buena sola.
    pub(in crate::site::application::errand) fn remember_the_confirmation(
        &self,
        consent: ConfirmationConsent,
    ) {
        *crate::lock(&self.consent) = Some(PendingConsent::Confirmation(consent));
    }

    /// Confirmación pendiente, si el trámite está esperando una.
    pub(in crate::site::application::errand) fn the_confirmation_pending(
        &self,
    ) -> Option<ConfirmationConsent> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Confirmation(consent)) => Some(consent.clone()),
            _ => None,
        }
    }

    /// Registra el lote pendiente de consentimiento o de postfirma.
    pub(in crate::site::application::errand) fn remember_the_batch(&self, pending: PendingBatch) {
        *crate::lock(&self.consent) = Some(PendingConsent::Batch(pending));
    }

    /// Si el trámite tiene una firma contra el servidor trifásico consentida esperando el secreto.
    pub fn a_server_signature_is_pending(&self) -> bool {
        matches!(
            &*crate::lock(&self.consent),
            Some(PendingConsent::Signature(pending))
                if pending.through_the_server.as_ref().is_some_and(|server| server.chosen.is_some())
        )
    }

    /// Si el trámite tiene un lote consentido esperando el secreto.
    pub fn a_batch_is_pending(&self) -> bool {
        matches!(
            &*crate::lock(&self.consent),
            Some(PendingConsent::Batch(pending)) if pending.chosen.is_some()
        )
    }

    /// El certificado consentido que espera el secreto sin ciclo abierto: el del lote, remoto o local, o el de la firma contra el servidor trifásico.
    pub fn the_certificate_awaiting_the_secret(&self) -> Option<TokenCertificate> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Batch(pending)) => pending.chosen.clone(),
            Some(PendingConsent::LocalBatch(pending)) => pending.chosen.clone(),
            Some(PendingConsent::Signature(pending)) => pending
                .through_the_server
                .as_ref()
                .and_then(|server| server.chosen.clone()),
            _ => None,
        }
    }

    /// Lote pendiente, si el trámite está atendiendo uno.
    pub(in crate::site::application::errand) fn the_batch_pending(&self) -> Option<PendingBatch> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Batch(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Registra el lote local pendiente de consentimiento o de firma.
    pub(in crate::site::application::errand) fn remember_the_local_batch(
        &self,
        pending: PendingLocalBatch,
    ) {
        *crate::lock(&self.consent) = Some(PendingConsent::LocalBatch(pending));
    }

    /// Lote local pendiente, si el trámite está atendiendo uno.
    pub(in crate::site::application::errand) fn the_local_batch_pending(
        &self,
    ) -> Option<PendingLocalBatch> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::LocalBatch(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Registra los datos del diálogo de guardado pendiente.
    pub(in crate::site::application::errand) fn remember_saving(&self, consent: SavingConsent) {
        *crate::lock(&self.consent) = Some(PendingConsent::Saving(consent));
    }

    /// Registra los datos del selector de carga pendiente.
    pub(in crate::site::application::errand) fn remember_loading(&self, consent: LoadingConsent) {
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
    pub(in crate::site::application::errand) fn what_the_site_asked(
        &self,
    ) -> Option<(SiteFilter, bool)> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Identity(filter, sticky)) => Some((filter.clone(), *sticky)),
            _ => None,
        }
    }

    /// Firma consentida pendiente, si la hay.
    pub(in crate::site::application::errand) fn the_signature_consented(
        &self,
    ) -> Option<PendingSignature> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Signature(pending)) if pending.area.is_none() => {
                Some(pending.clone())
            }
            _ => None,
        }
    }

    /// Registra el rechazo que la ventana enseña antes de contestarlo.
    pub(in crate::site::application::errand) fn remember_the_refusal(&self, refusal: Refusal) {
        *crate::lock(&self.consent) = Some(PendingConsent::ShownRefusal(refusal));
    }

    /// El rechazo que la ventana enseña y la sede aún no ha recibido, si lo hay.
    pub(in crate::site::application::errand) fn the_shown_refusal(&self) -> Option<Refusal> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::ShownRefusal(refusal)) => Some(refusal.clone()),
            _ => None,
        }
    }

    /// Limpia los datos de consentimiento registrados.
    pub(in crate::site::application::errand) fn forget_the_consent(&self) {
        *crate::lock(&self.consent) = None;
    }
}
