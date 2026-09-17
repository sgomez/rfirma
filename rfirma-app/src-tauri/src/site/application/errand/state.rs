//! Estado del trámite con la sede y gestión de su ciclo de vida (ADR-0016).

use crate::site::application::startup::SiteWindow;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::identity::domain::certificate::TokenCertificate;
use crate::signing::domain::bridge::{Format, SignatureOperation};
use crate::site::domain::batch::LocalBatch;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::{
    AfirmaUrl, AskedAlgorithm, BatchRequest, NegotiatedCredential, SiteFilter,
};

use super::outcome::{
    ConfirmationConsent, LoadingConsent, Moment, ProtocolCodec, SavingConsent, SavingHints,
    SiteOutcome,
};
use crate::site::ports::{Acknowledgement, ReplyHandle, Scratch};

struct RevelationInner {
    revealed: bool,
    cancelled: bool,
}

/// Qué hacer con la ventana cuando la espera de respaldo se cumple, por revelación o por plazo.
#[derive(Clone, Copy)]
enum RevelationAction {
    /// Revela la ventana del trámite que sigue esperando al navegador.
    Show,
    /// Cierra la ventana oculta que sostenía un rechazo retenido por el canal.
    EndTheErrand,
}

#[derive(Clone)]
struct RevelationHandle {
    state: Arc<(Mutex<RevelationInner>, Condvar)>,
    window: Arc<dyn SiteWindow>,
    action: RevelationAction,
}

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
}

/// Datos identificativos y de conexión de un trámite en curso.
#[derive(Clone)]
pub struct Errand {
    credential: NegotiatedCredential,
    arrival: ArrivalMode,
    codec: NegotiatedCodec,
}

impl std::fmt::Debug for Errand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Errand")
            .field("credential", &self.credential)
            .field("arrival", &self.arrival)
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
        }
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
        self.cancel_backing_timeout();
        *crate::lock(&self.errand) = None;
        drop(crate::lock(&self.reply).take());
        if let Some(scratch) = crate::lock(&self.scratch).take() {
            scratch.files.erase(&scratch.path);
        }
        *crate::lock(&self.asked) = None;
        self.forget_the_consent();

        if let Some(window) = crate::lock(&self.window).take() {
            let delivered = crate::lock(&self.delivered)
                .take()
                .unwrap_or_else(Acknowledgement::immediate);
            window.errand_ended(delivered);
        }
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

    /// Si el trámite tiene un lote consentido esperando el secreto.
    pub fn a_batch_is_pending(&self) -> bool {
        matches!(
            &*crate::lock(&self.consent),
            Some(PendingConsent::Batch(pending)) if pending.chosen.is_some()
        )
    }

    /// El certificado consentido del lote, remoto o local, que espera el secreto.
    pub fn the_batch_certificate(&self) -> Option<TokenCertificate> {
        match &*crate::lock(&self.consent) {
            Some(PendingConsent::Batch(pending)) => pending.chosen.clone(),
            Some(PendingConsent::LocalBatch(pending)) => pending.chosen.clone(),
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

    /// Limpia los datos de consentimiento registrados.
    pub(super) fn forget_the_consent(&self) {
        *crate::lock(&self.consent) = None;
    }

    /// Arma el temporizador de respaldo para revelar la ventana si el navegador no llega.
    pub fn arm_backing_timeout(&self, window: Arc<dyn SiteWindow>, threshold: Duration) {
        self.arm_expiring_wait(window, threshold, RevelationAction::Show);
    }

    /// Arma la espera de que se sirva un rechazo retenido por el canal, cerrando la ventana
    /// oculta que lo sostiene al cumplirse o al vencer el plazo.
    pub fn arm_channel_refusal_wait(&self, window: Arc<dyn SiteWindow>, threshold: Duration) {
        self.arm_expiring_wait(window, threshold, RevelationAction::EndTheErrand);
    }

    fn arm_expiring_wait(
        &self,
        window: Arc<dyn SiteWindow>,
        threshold: Duration,
        action: RevelationAction,
    ) {
        self.cancel_backing_timeout();
        let state = Arc::new((
            Mutex::new(RevelationInner {
                revealed: false,
                cancelled: false,
            }),
            Condvar::new(),
        ));
        let handle = RevelationHandle {
            state: Arc::clone(&state),
            window: Arc::clone(&window),
            action,
        };
        *crate::lock(&self.revelation) = Some(handle);

        let timer_state = Arc::clone(&state);
        let timer_window = Arc::clone(&window);
        let timer_moment = Arc::clone(&self.moment);

        std::thread::spawn(move || {
            let (lock, cvar) = &*timer_state;
            let mut inner = lock.lock().unwrap();
            while !inner.cancelled && !inner.revealed {
                let result = cvar.wait_timeout(inner, threshold).unwrap();
                inner = result.0;
                if result.1.timed_out() {
                    break;
                }
            }
            if !inner.cancelled && !inner.revealed {
                inner.revealed = true;
                drop(inner);
                match action {
                    RevelationAction::Show => {
                        *timer_moment.lock().unwrap() = Some(Moment::Unreachable);
                        timer_window.show();
                    }
                    RevelationAction::EndTheErrand => {
                        timer_window.errand_ended(Acknowledgement::immediate());
                    }
                }
            }
        });
    }

    /// Cancela el temporizador de respaldo si estaba activo.
    pub fn cancel_backing_timeout(&self) {
        if let Some(handle) = crate::lock(&self.revelation).take() {
            let (lock, cvar) = &*handle.state;
            let mut inner = lock.lock().unwrap();
            inner.cancelled = true;
            cvar.notify_all();
        }
    }

    /// Notifica que el navegador ha llegado al canal, revelando la ventana o cerrándola,
    /// según lo que se armó.
    pub fn browser_arrived(&self) {
        let handle = crate::lock(&self.revelation).as_ref().cloned();
        if let Some(handle) = handle {
            let (lock, cvar) = &*handle.state;
            let mut inner = lock.lock().unwrap();
            if !inner.revealed && !inner.cancelled {
                inner.revealed = true;
                cvar.notify_all();
                drop(inner);
                match handle.action {
                    RevelationAction::Show => handle.window.show(),
                    RevelationAction::EndTheErrand => {
                        handle.window.errand_ended(Acknowledgement::immediate());
                    }
                }
            }
        }
    }

    /// Comprueba si la ventana del trámite ha sido revelada.
    pub fn is_revealed(&self) -> bool {
        crate::lock(&self.revelation)
            .as_ref()
            .map(|r| r.state.0.lock().unwrap().revealed)
            .unwrap_or(false)
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
