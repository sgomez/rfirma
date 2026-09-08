//! Contexto `site` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::path::PathBuf;
use std::sync::Arc;

use crate::crossing::Failure;
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
    /// Los dos servlets del lote remoto.
    pub batch: Arc<dyn ports::BatchServices + Send + Sync>,
}

/// Cierra el lote pendiente, remoto o local, con el secreto que entró por la única puerta del PIN.
pub fn the_pending_batch_signed(
    app: &tauri::AppHandle,
    secret: &str,
) -> Option<Result<(), Failure>> {
    use tauri::Manager as _;

    let root = app.state::<SiteRoot>();
    if root.errand.a_batch_is_pending() {
        return Some(
            adapters::window::with_the_desk(app, |desk, live| {
                application::errand::finish_the_batch(desk, secret, live)
            })
            .map_err(Failure::from),
        );
    }
    if root.errand.a_local_batch_is_pending() {
        return Some(
            adapters::window::with_the_desk(app, |_desk, live| {
                application::errand::finish_the_local_batch(secret, live)
            })
            .map_err(Failure::from),
        );
    }
    None
}
