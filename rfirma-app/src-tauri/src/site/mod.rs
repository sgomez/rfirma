//! Contexto `site` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::PathBuf;
use std::sync::Arc;

use crate::crossing::Failure;
use crate::identity::domain::protected_secret::ProtectedSecret;
use crate::signing::adapters::isolate::Isolate;
use adapters::tls::LocalCaStore;
use application::errand::ErrandDesk;
pub use application::errand::LiveErrand;
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
    /// Los dos servlets del lote remoto.
    pub batch: Arc<dyn ports::BatchServices + Send + Sync>,
}

/// La mesa del trámite sobre las raíces de producción.
pub type SiteDesk<'a> = ErrandDesk<'a, Isolate, Isolate, adapters::desk::Neighbours<'a>>;

/// Cierra el lote consentido, remoto o local, con el secreto que entró por la única puerta del PIN.
pub fn the_pending_batch_signed(
    desk: &SiteDesk<'_>,
    live: &LiveErrand,
    secret: &ProtectedSecret,
) -> Result<(), Failure> {
    let secret = secret
        .expose_secret()
        .map_err(|_| Failure::new("unknown", "el secreto tecleado no es texto válido"))?;
    if live.a_batch_is_pending() {
        return application::errand::finish_the_batch(desk, secret, live).map_err(Failure::from);
    }
    application::errand::finish_the_local_batch(desk, secret, live).map_err(Failure::from)
}
