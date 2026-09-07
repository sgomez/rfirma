//! Contexto `signing` (ADR-0017): la raíz de composición y lo que presta a los vecinos.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::documents::domain::document::Document;
use crate::identity::domain::certificate::TokenCertificate;
use crate::identity::domain::secret::StoreSecret;
use adapters::isolate::Isolate;
use adapters::memory::Memory;
use application::configuration_memory::Configuration;
use application::session::{CycleFailure, DocumentToSign, Signed, SigningSession};
use ports::Signer;

/// La raíz de `signing`: la memoria entre sesiones, el hilo del aislado y la sesión de firma.
pub struct SigningRoot {
    /// Las dos memorias y la configuración viva (ADR-0010).
    pub memory: Arc<Memory>,
    /// El hilo dueño del isolate.
    pub isolate: Isolate,
    /// La sesión de firma a medias.
    pub session: SigningSession,
}

impl SigningRoot {
    /// Una instantánea de la configuración viva.
    pub fn configuration(&self) -> Configuration {
        self.memory.configuration()
    }

    /// Si hay una firma a medias.
    pub fn is_live(&self) -> bool {
        application::session::is_live(&self.session)
    }

    /// Ruta del último documento firmado entregado en esta sesión (ADR-0011).
    pub fn signed_document(&self) -> Result<PathBuf, CycleFailure> {
        application::session::signed_document(&self.session)
    }

    /// Directorio del último documento firmado entregado en esta sesión (ADR-0011).
    pub fn signed_folder(&self) -> Result<PathBuf, CycleFailure> {
        application::session::signed_folder(&self.session)
    }

    /// Prefirma de un trámite de sede sobre el documento de paso y el certificado ya cribado.
    pub fn begin_for_the_site(
        &self,
        handle: &str,
        document: Document,
        chosen: &TokenCertificate,
        from_the_site: &BTreeMap<String, String>,
        allow_unregistered_signatures: bool,
        signer: &dyn Signer,
    ) -> Result<StoreSecret, CycleFailure> {
        application::session::begin_for_the_site(
            DocumentToSign {
                handle: handle.to_owned(),
                document,
            },
            chosen,
            from_the_site,
            allow_unregistered_signatures,
            signer,
            &self.isolate,
            &self.session,
        )
    }

    /// Postfirma: el ciclo completado, sin entregar nada.
    pub fn finish(&self) -> Result<Signed, CycleFailure> {
        application::session::finish(&self.isolate, &self.session)
    }
}
