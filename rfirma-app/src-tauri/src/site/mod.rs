//! Contexto `site` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::PathBuf;
use std::sync::Arc;

use adapters::tls::LocalCaStore;
use application::errand::LiveErrand;
use application::site::CodecTable;
use application::startup::{HeldChannel, LocalCaTrust};

/// La raíz de `site`: el trámite vivo, el canal sostenido, la confianza de la CA local y la tabla
/// de códecs.
pub struct SiteRoot {
    /// El trámite vivo del proceso.
    pub errand: LiveErrand,
    /// El canal abierto con la sede, sostenido.
    pub held_channel: HeldChannel,
    /// La CA local, sus almacenes y los perfiles NSS.
    pub trust: LocalCaTrust,
    /// Las dos ranuras de la CA local en disco.
    pub ca_store: LocalCaStore,
    /// La tabla de códecs que la negociación elige según la forma de la invocación.
    pub codecs: CodecTable,
    /// Directorio para los documentos de paso.
    pub scratch_dir: PathBuf,
    /// Quien escribe y borra el fichero de paso.
    pub scratch: Arc<dyn ports::Scratch + Send + Sync>,
}
