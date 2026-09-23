//! Revelación de la ventana: por temporizador de respaldo, por llegada del navegador o porque el trámite tiene algo que decir.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::site::application::errand::outcome::Moment;
use crate::site::application::startup::SiteWindow;
use crate::site::domain::channel::ArrivalMode;
use crate::site::domain::protocol::Refusal;
use crate::site::ports::Acknowledgement;

use super::LiveErrand;

pub(super) struct RevelationInner {
    revealed: bool,
    cancelled: bool,
}

/// Qué hacer con la ventana cuando la espera de respaldo se cumple, por revelación o por plazo.
#[derive(Clone, Copy)]
pub(super) enum RevelationAction {
    /// Revela la ventana del trámite que sigue esperando al navegador.
    Show,
    /// Cierra la ventana oculta que sostenía un rechazo retenido por el canal.
    EndTheErrand,
}

#[derive(Clone)]
pub(super) struct RevelationHandle {
    state: Arc<(Mutex<RevelationInner>, Condvar)>,
    window: Arc<dyn SiteWindow>,
    action: RevelationAction,
}

impl LiveErrand {
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
        self.arrived
            .store(true, std::sync::atomic::Ordering::SeqCst);
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

    /// Enseña la ventana de un trámite sin navegador que esperar, porque su operación tiene algo que decir.
    pub(in crate::site::application::errand) fn reveal_an_immediate_arrival(&self) {
        let immediate = self
            .current()
            .is_some_and(|errand| errand.arrival() == ArrivalMode::Immediate);
        if let (true, Some(window)) = (immediate, self.the_window()) {
            window.show();
        }
    }

    /// Enseña a la persona el rechazo con el que la respuesta no llegó a la sede.
    pub fn the_site_did_not_get_the_answer(&self, refusal: Refusal) {
        self.note(Moment::RefusedWithoutChannel(refusal));
        if let Some(window) = self.the_window() {
            window.show();
        }
    }

    /// Comprueba si la ventana del trámite ha sido revelada.
    pub fn is_revealed(&self) -> bool {
        crate::lock(&self.revelation)
            .as_ref()
            .map(|r| r.state.0.lock().unwrap().revealed)
            .unwrap_or(false)
    }
}
