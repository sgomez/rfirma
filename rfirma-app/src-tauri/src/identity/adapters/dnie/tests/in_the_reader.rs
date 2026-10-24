use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use rdnie::{Session, Transport, SIGNATURE_LABEL};
use rdnie_emulator::{EmulatedDnie, DEFAULT_PIN};

use super::super::{dnie_store, CardOpener, DnieToken};
use crate::identity::domain::certificate::CertificateRef;
use crate::identity::domain::protected_secret::ProtectedSecret;

/// Un solo DNIe emulado que sigue en el lector entre aperturas, y cuántas sesiones lo tienen abierto.
#[derive(Clone)]
pub(super) struct InTheReader {
    card: Arc<Mutex<EmulatedDnie>>,
    sessions: Arc<AtomicUsize>,
    openings: Arc<AtomicUsize>,
    taken_out: Arc<AtomicBool>,
}

impl InTheReader {
    pub(super) fn new(card: EmulatedDnie) -> Self {
        Self {
            card: Arc::new(Mutex::new(card)),
            sessions: Arc::new(AtomicUsize::new(0)),
            openings: Arc::new(AtomicUsize::new(0)),
            taken_out: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Saca la tarjeta del lector, como lo vería PC/SC.
    pub(super) fn take_the_card_out(&self) {
        self.card().remove_card();
        self.taken_out.store(true, Ordering::SeqCst);
    }

    pub(super) fn card(&self) -> MutexGuard<'_, EmulatedDnie> {
        self.card
            .lock()
            .expect("la tarjeta no deberia quedar envenenada")
    }

    pub(super) fn token(&self) -> DnieToken<Self> {
        DnieToken::new(self.clone())
    }

    pub(super) fn open_sessions(&self) -> usize {
        self.sessions.load(Ordering::SeqCst)
    }

    pub(super) fn openings(&self) -> usize {
        self.openings.load(Ordering::SeqCst)
    }

    pub(super) fn pins_sent(&self) -> usize {
        self.card()
            .pin_queries()
            .iter()
            .filter(|query| query.with_pin)
            .count()
    }
}

/// Una sesión con la tarjeta de [`InTheReader`], que se cuenta mientras vive.
pub(super) struct Connection(InTheReader);

impl Drop for Connection {
    fn drop(&mut self) {
        self.0.sessions.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Transport for Connection {
    fn transmit_raw(&mut self, apdu: &[u8]) -> Result<Vec<u8>, rdnie::Error> {
        self.0.card().transmit_raw(apdu)
    }

    fn atr(&self) -> Result<Vec<u8>, rdnie::Error> {
        self.0.card().atr()
    }

    fn reset(&mut self) -> Result<Vec<u8>, rdnie::Error> {
        self.0.card().reset()
    }

    fn transaction<R>(
        &mut self,
        body: impl FnOnce(&mut dyn Transport) -> Result<R, rdnie::Error>,
    ) -> Result<R, rdnie::Error> {
        self.0.card().transaction(body)
    }
}

impl CardOpener for InTheReader {
    type Card = Connection;

    fn open(&self) -> Result<Session<Connection>, rdnie::Error> {
        self.sessions.fetch_add(1, Ordering::SeqCst);
        self.openings.fetch_add(1, Ordering::SeqCst);
        let root = self.card().component_root();
        Session::open_trusting(Connection(self.clone()), root)
    }

    fn insertion(&self) -> Option<u64> {
        Some(u64::from(self.taken_out.load(Ordering::SeqCst)))
    }
}

pub(super) fn the_pin() -> ProtectedSecret {
    ProtectedSecret::from_str(DEFAULT_PIN)
}

pub(super) fn a_wrong_pin() -> ProtectedSecret {
    ProtectedSecret::from_str("OtroPin9")
}

/// El certificado de firma del DNIe, nombrado sin leer la tarjeta.
pub(super) fn the_signing_certificate() -> CertificateRef {
    CertificateRef::new(dnie_store(), "DNIe", SIGNATURE_LABEL, None)
}
