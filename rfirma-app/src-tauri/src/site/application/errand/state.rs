//! Estado del trámite con la sede, con un solo dueño (`LiveErrand`), y gestión de su ciclo de vida (ADR-0016).

mod area;
mod chosen_document;
mod consent;
mod revelation;

use crate::site::application::startup::{HeldLaunch, SiteWindow};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::identity::domain::certificate::CertificateRef;
use crate::site::domain::channel::{ArrivalMode, ChannelTenure};
use crate::site::domain::protocol::{AfirmaUrl, NegotiatedCredential};
use crate::site::domain::site_origin::SiteOrigin;

use super::outcome::{Moment, ProtocolCodec, SiteOutcome};
use crate::site::ports::{Acknowledgement, ReplyHandle, Scratch};
pub(super) use area::AreaToMark;
use consent::PendingConsent;
pub(super) use consent::{PendingBatch, PendingLocalBatch, PendingSignature, ServerSignature};
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
    scratch: Mutex<Vec<KeptScratch>>,
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
    chosen_document: Mutex<Option<String>>,
    origin: Mutex<SiteOrigin>,
    sha1_once: std::sync::atomic::AtomicBool,
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

impl LiveErrand {
    /// Trámite de prueba inicializado con un códec específico.
    #[cfg(test)]
    pub fn speaking(codec: NegotiatedCodec) -> Self {
        let live = Self::default();
        *crate::lock(&live.codec) = Some(codec);
        live
    }

    /// Trámite de prueba con un trámite ya activo, para probar qué ve `current()`.
    #[cfg(test)]
    pub fn speaking_with_an_active_errand(codec: NegotiatedCodec) -> Self {
        let live = Self::speaking(Arc::clone(&codec));
        let _ = live.begin(Errand::of(
            NegotiatedCredential::Absent,
            ArrivalMode::Awaited,
            codec,
        ));
        live
    }

    /// Registra la ruta temporal de un fichero de paso, que se borra al acabar la operación.
    pub(super) fn keep_the_scratch(&self, path: PathBuf, files: Arc<dyn Scratch + Send + Sync>) {
        crate::lock(&self.scratch).push(KeptScratch { path, files });
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
        *crate::lock(&self.origin) = SiteOrigin::absent();
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

    /// Registra el origen de la operación que atiende, sustituyendo al de la anterior.
    pub fn note_origin(&self, origin: SiteOrigin) {
        *crate::lock(&self.origin) = origin;
    }

    /// Origen de la operación que atiende, o su ausencia mientras no haya llegado ninguna.
    pub fn origin(&self) -> SiteOrigin {
        crate::lock(&self.origin).clone()
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
        for scratch in std::mem::take(&mut *crate::lock(&self.scratch)) {
            scratch.files.erase(&scratch.path);
        }
        *crate::lock(&self.asked) = None;
        *crate::lock(&self.chosen_document) = None;
        self.sha1_once
            .store(false, std::sync::atomic::Ordering::SeqCst);
        self.forget_the_consent();
    }

    /// Permite SHA-1 fuera de XML solo en la operación en curso, sin tocar la preferencia (ADR-0023).
    pub fn allow_sha1_once(&self) {
        self.sha1_once
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Si la persona permitió SHA-1 en la operación en curso.
    pub fn sha1_allowed_once(&self) -> bool {
        self.sha1_once.load(std::sync::atomic::Ordering::SeqCst)
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

    /// Ruta al primer fichero de paso de la operación, para pruebas.
    #[cfg(test)]
    pub fn scratch_path(&self) -> Option<PathBuf> {
        crate::lock(&self.scratch)
            .first()
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
