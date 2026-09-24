//! Estado del trámite con la sede y gestión de su ciclo de vida (ADR-0016).

mod revelation;

use crate::site::application::startup::{HeldLaunch, SiteWindow};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::identity::domain::certificate::{CertificateRef, TokenCertificate};
use crate::signing::domain::bridge::{Format, SignatureOperation};
use crate::site::domain::batch::LocalBatch;
use crate::site::domain::channel::{ArrivalMode, ChannelTenure};
use crate::site::domain::protocol::{
    AfirmaUrl, AskedAlgorithm, BatchRequest, NegotiatedCredential, Refusal, SignatureRound,
    SiteFilter,
};
use crate::site::domain::signing::SiteSignature;
use crate::site::domain::triphase_server::ServerFormat;

use super::outcome::{
    ConfirmationConsent, LoadingConsent, Moment, ProtocolCodec, SavingConsent, SavingHints,
    SiteOutcome,
};
use crate::site::ports::{Acknowledgement, ReplyHandle, Scratch};
use revelation::RevelationHandle;

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
    moment: Arc<Mutex<Option<Moment>>>,
    revelation: Mutex<Option<RevelationHandle>>,
    window: Mutex<Option<Arc<dyn SiteWindow>>>,
    delivered: Mutex<Option<Acknowledgement>>,
    stuck: Mutex<Option<CertificateRef>>,
    arrived: std::sync::atomic::AtomicBool,
    held_launch: Mutex<Option<HeldLaunch>>,
}

/// Datos identificativos y de conexión de un trámite en curso.
#[derive(Clone)]
pub struct Errand {
    credential: NegotiatedCredential,
    arrival: ArrivalMode,
    codec: NegotiatedCodec,
    tenure: ChannelTenure,
}

impl std::fmt::Debug for Errand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Errand")
            .field("credential", &self.credential)
            .field("arrival", &self.arrival)
            .field("tenure", &self.tenure)
            .finish_non_exhaustive()
    }
}

impl Errand {
    /// Construye un trámite con la credencial, llegada y códec indicados.
    pub fn of(
        credential: NegotiatedCredential,
        arrival: ArrivalMode,
        codec: NegotiatedCodec,
    ) -> Self {
        Self {
            credential,
            arrival,
            codec,
            tenure: ChannelTenure::OneOperation,
        }
    }

    /// El mismo trámite, atendiendo tantas operaciones como diga su canal.
    pub fn with_tenure(self, tenure: ChannelTenure) -> Self {
        Self { tenure, ..self }
    }

    /// Cuántas operaciones atiende este trámite.
    pub fn tenure(&self) -> ChannelTenure {
        self.tenure
    }

    /// Credencial con la que se cerró el canal, si la sede la exigió.
    pub fn credential(&self) -> &NegotiatedCredential {
        &self.credential
    }

    /// Modo de llegada con el que se abrió el canal del trámite.
    pub fn arrival(&self) -> ArrivalMode {
        self.arrival
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
    Confirmation(ConfirmationConsent),
    Batch(PendingBatch),
    LocalBatch(PendingLocalBatch),
    Saving(SavingConsent),
    Loading(LoadingConsent),
    ShownRefusal(Refusal),
}

/// Lo que el lote remoto necesita entre el consentimiento y la postfirma.
#[derive(Clone, Debug)]
pub(super) struct PendingBatch {
    /// El lote tal y como lo pidió la sede.
    pub(super) request: BatchRequest,
    /// El certificado que la persona consintió, una vez consentido.
    pub(super) chosen: Option<TokenCertificate>,
}

/// Lo que el lote local necesita entre el consentimiento y el bucle de firma.
#[derive(Clone, Debug)]
pub(super) struct PendingLocalBatch {
    /// El lote tal y como lo pidió la sede.
    pub(super) request: BatchRequest,
    /// Las firmas del lote, ya leídas.
    pub(super) batch: LocalBatch,
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
    /// Huella que pidió la sede para esta firma.
    pub(super) algorithm: AskedAlgorithm,
    /// Qué pidió hacer la sede con el documento.
    pub(super) operation: SignatureOperation,
    /// Parámetros adicionales expandidos.
    pub(super) from_the_site: BTreeMap<String, String>,
    /// Si el documento contiene firmas no reconocidas.
    pub(super) unregistered_signatures: bool,
    /// Pistas de guardado, si esta firma viene de `signandsave`.
    pub(super) saving: Option<Box<SavingHints>>,
    /// La firma que hace el servidor trifásico de la sede, si se hace allí.
    pub(super) through_the_server: Option<ServerSignature>,
}

/// Lo que la firma contra el servidor trifásico lleva del consentimiento a la entrega.
#[derive(Clone, Debug)]
pub(super) struct ServerSignature {
    /// El firmador trifásico que eligió la sede.
    pub(super) format: ServerFormat,
    /// Los datos, o la firma previa en cofirma y contrafirma.
    pub(super) document: Vec<u8>,
    /// La operación que pidió la sede.
    pub(super) round: SignatureRound,
    /// El certificado que la persona consintió, una vez consentido.
    pub(super) chosen: Option<TokenCertificate>,
    /// La firma que devolvió la postfirma, una vez hecha.
    pub(super) signed: Option<SiteSignature>,
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
        self.cancel_backing_timeout();
        *crate::lock(&self.codec) = Some(Arc::clone(&errand.codec));
        *live = Some(errand);
        true
    }

    /// Registra el asa de respuesta para contestar a la sede.
    pub fn answer_through(&self, reply: ReplyHandle) {
        *crate::lock(&self.reply) = Some(reply);
    }

    /// Registra la ventana que hay que avisar cuando este trámite termine.
    pub fn keep_the_window(&self, window: Arc<dyn SiteWindow>) {
        *crate::lock(&self.window) = Some(window);
    }

    /// Envía la respuesta codificada a la sede a través del asa.
    pub(super) fn answer_the_site(&self, outcome: &SiteOutcome) {
        let Some(reply) = crate::lock(&self.reply).take() else {
            return;
        };
        if let Some(codec) = self.codec() {
            let acknowledgement = reply.answer(codec.encode(outcome));
            *crate::lock(&self.delivered) = Some(acknowledgement);
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

    /// Espera hasta el tope al acuse de entrega ya registrado por `answer_the_site`, si lo hay.
    pub(super) fn wait_for_delivery(&self, timeout: Duration) {
        if let Some(delivered) = crate::lock(&self.delivered).take() {
            delivered.wait(timeout);
        }
    }

    /// Finaliza el trámite y limpia sus recursos asociados; si había una ventana, se le avisa.
    pub fn end(&self) {
        if self.keeps_serving() {
            self.end_the_operation();
            drop(crate::lock(&self.delivered).take());
            return;
        }
        self.cancel_backing_timeout();
        *crate::lock(&self.errand) = None;
        self.end_the_operation();

        if let Some(window) = crate::lock(&self.window).take() {
            let delivered = crate::lock(&self.delivered)
                .take()
                .unwrap_or_else(Acknowledgement::immediate);
            window.errand_ended(delivered);
        }
    }

    fn end_the_operation(&self) {
        drop(crate::lock(&self.reply).take());
        if let Some(scratch) = crate::lock(&self.scratch).take() {
            scratch.files.erase(&scratch.path);
        }
        *crate::lock(&self.asked) = None;
        self.forget_the_consent();
    }

    fn serves_many_operations(&self) -> bool {
        crate::lock(&self.errand)
            .as_ref()
            .is_some_and(|errand| errand.tenure != ChannelTenure::OneOperation)
    }

    fn the_window(&self) -> Option<Arc<dyn SiteWindow>> {
        crate::lock(&self.window).clone()
    }

    /// Si cerrar la ventana solo la oculta, porque el primer cliente sigue pudiendo pedir más.
    pub fn keeps_serving(&self) -> bool {
        self.serves_many_operations() && self.arrived.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Termina el trámite de WebSocket porque se ha ido su primer cliente, y cierra su ventana.
    pub fn the_first_client_left(&self) {
        self.let_the_channel_go();
    }

    /// Termina el trámite de `service` porque su canal ha vencido sin órdenes, y cierra su ventana.
    pub fn the_channel_went_idle(&self) {
        self.let_the_channel_go();
    }

    fn let_the_channel_go(&self) {
        if !self.serves_many_operations() {
            return;
        }
        self.cancel_backing_timeout();
        *crate::lock(&self.errand) = None;
        self.end_the_operation();
        drop(crate::lock(&self.delivered).take());
        if let Some(window) = crate::lock(&self.window).take() {
            window.close();
        }
    }

    /// Vuelve a enseñar la ventana en la espera, como con la primera operación del canal.
    pub(super) fn an_operation_arrives(&self) {
        if !self.serves_many_operations() {
            return;
        }
        self.note(Moment::Waiting);
        if let Some(window) = self.the_window() {
            window.show();
        }
    }

    /// Oculta la ventana y termina la operación antes de contestar, para que la siguiente no llegue a una ventana que se oculta después.
    pub(super) fn answer_once_put_away(&self, outcome: &SiteOutcome) {
        let reply = crate::lock(&self.reply).take();
        self.put_away_the_window();
        self.end_the_operation();
        drop(crate::lock(&self.delivered).take());
        if let (Some(reply), Some(codec)) = (reply, self.codec()) {
            drop(reply.answer(codec.encode(outcome)));
        }
    }

    /// Retiene el arranque hasta que la persona descarte el aviso que lo precede.
    pub fn hold_back(&self, launch: HeldLaunch) {
        *crate::lock(&self.held_launch) = Some(launch);
    }

    /// Si hay un arranque retenido tras un aviso.
    pub fn holds_back_a_launch(&self) -> bool {
        crate::lock(&self.held_launch).is_some()
    }

    pub(super) fn take_the_held_launch(&self) -> Option<HeldLaunch> {
        crate::lock(&self.held_launch).take()
    }

    /// Oculta la ventana de una operación contestada sin nada que enseñar.
    pub(super) fn put_away_the_window(&self) {
        if !self.serves_many_operations() {
            return;
        }
        if let Some(window) = self.the_window() {
            window.hide();
        }
    }

    /// Ruta al fichero temporal para pruebas.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        crate::lock(&self.scratch)
            .as_ref()
            .map(|scratch| scratch.path.clone())
    }

    /// Fija para `sticky` el certificado elegido, solo en esta sesión de sede.
    pub(super) fn stick(&self, chosen: &CertificateRef) {
        *crate::lock(&self.stuck) = Some(chosen.clone());
    }

    /// El certificado que `sticky` fijó en esta sesión, si lo hay.
    pub(super) fn the_stuck(&self) -> Option<CertificateRef> {
        crate::lock(&self.stuck).clone()
    }

    /// Olvida el certificado fijado en esta sesión.
    pub(super) fn unstick(&self) {
        *crate::lock(&self.stuck) = None;
    }

    /// Registra el filtro de consentimiento de identidad y si la sede lo pegó.
    pub(super) fn remember_identity(&self, filter: SiteFilter, sticky: bool) {
        *crate::lock(&self.consent) = Some(PendingConsent::Identity(filter, sticky));
    }

    /// Registra los datos de consentimiento de firma.
    pub(super) fn remember_signature(&self, pending: PendingSignature) {
        *crate::lock(&self.consent) = Some(PendingConsent::Signature(pending));
    }

    /// Registra la confirmación pendiente de una firma que el validador no da por buena sola.
    pub(super) fn remember_the_confirmation(&self, consent: ConfirmationConsent) {
        *crate::lock(&self.consent) = Some(PendingConsent::Confirmation(consent));
    }

    /// Confirmación pendiente, si el trámite está esperando una.
    pub(super) fn the_confirmation_pending(&self) -> Option<ConfirmationConsent> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Confirmation(consent)) => Some(consent.clone()),
            _ => None,
        }
    }

    /// Registra el lote pendiente de consentimiento o de postfirma.
    pub(super) fn remember_the_batch(&self, pending: PendingBatch) {
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
    pub(super) fn the_batch_pending(&self) -> Option<PendingBatch> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Batch(pending)) => Some(pending.clone()),
            _ => None,
        }
    }

    /// Registra el lote local pendiente de consentimiento o de firma.
    pub(super) fn remember_the_local_batch(&self, pending: PendingLocalBatch) {
        *crate::lock(&self.consent) = Some(PendingConsent::LocalBatch(pending));
    }

    /// Lote local pendiente, si el trámite está atendiendo uno.
    pub(super) fn the_local_batch_pending(&self) -> Option<PendingLocalBatch> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::LocalBatch(pending)) => Some(pending.clone()),
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

    /// Registra el rechazo que la ventana enseña antes de contestarlo.
    pub(super) fn remember_the_refusal(&self, refusal: Refusal) {
        *crate::lock(&self.consent) = Some(PendingConsent::ShownRefusal(refusal));
    }

    /// El rechazo que la ventana enseña y la sede aún no ha recibido, si lo hay.
    pub(super) fn the_shown_refusal(&self) -> Option<Refusal> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::ShownRefusal(refusal)) => Some(refusal.clone()),
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
