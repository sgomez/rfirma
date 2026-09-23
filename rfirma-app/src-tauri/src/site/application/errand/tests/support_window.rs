//! La ventana doblada y la mesa desnuda con las que se prueba cuándo se enseña un trámite.

use crate::documents::application::documents::OpenedDocuments;
use crate::identity::application::certificates::ListedCertificates;
use crate::signing::application::tests::a_memory;
use crate::site::application::errand::*;
use crate::site::application::startup::{SiteWindow, SiteWindowContent};
use crate::site::domain::protocol::AfirmaUrl;

use super::support::{a_desk, APolicyEngine, AnEngine};

/// Ventana doblada que apunta lo que el trámite le pide.
#[derive(Default)]
pub(crate) struct AWindow {
    asked: std::sync::Mutex<Vec<&'static str>>,
}

impl AWindow {
    pub(crate) fn asked(&self) -> Vec<&'static str> {
        self.asked
            .lock()
            .expect("el doble no envenena su cerrojo")
            .clone()
    }

    pub(crate) fn note(&self, what: &'static str) {
        self.asked
            .lock()
            .expect("el doble no envenena su cerrojo")
            .push(what);
    }
}

impl SiteWindow for AWindow {
    fn open(&self, _content: SiteWindowContent<'_>) {
        self.note("abierta");
    }
    fn show(&self) {
        self.note("enseñada");
    }
    fn hide(&self) {
        self.note("oculta");
    }
    fn close(&self) {
        self.note("cerrada");
    }
    fn errand_ended(&self, _delivered: Acknowledgement) {
        self.note("trámite-terminado");
    }
}

/// Atiende la operación sobre una mesa sin certificados ni motores que contesten.
pub(crate) fn attended_on_a_bare_desk(
    url: AfirmaUrl,
    reply: ReplyHandle,
    live: &LiveErrand,
) -> ErrandStep {
    let home = tempfile::tempdir().expect("hay directorio temporal");
    let memory = a_memory(home.path());
    let listed = ListedCertificates::new();
    let opened_documents = OpenedDocuments::new();
    let engine = AnEngine::answering(&[]);
    let policies = APolicyEngine::answering("");
    let scratch = home.path().join("errand");
    let desk = a_desk(
        &engine,
        &policies,
        &[],
        home.path(),
        &listed,
        &opened_documents,
        &memory,
        &scratch,
    );
    attend(&desk, url, reply, live).expect("hay codec")
}
