//! Contexto `site` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::PathBuf;

use adapters::tls::LocalCaStore;
use application::errand::{LiveErrand, NegotiatedCodec};
use application::startup::{HeldChannel, LocalCaTrust};

/// La raíz de `site`: el trámite vivo, el canal sostenido, la confianza de la CA local y el códec.
pub struct SiteRoot {
    /// El trámite vivo del proceso.
    pub errand: LiveErrand,
    /// El canal abierto con la sede, sostenido.
    pub held_channel: HeldChannel,
    /// La CA local, sus almacenes y los perfiles NSS.
    pub trust: LocalCaTrust,
    /// Las dos ranuras de la CA local en disco.
    pub ca_store: LocalCaStore,
    /// El códec negociado con las sedes.
    pub codec: NegotiatedCodec,
    /// Directorio para los documentos de paso.
    pub scratch_dir: PathBuf,
}
