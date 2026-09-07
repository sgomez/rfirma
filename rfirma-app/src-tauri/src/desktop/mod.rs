//! Contexto `desktop` (ADR-0017): la raíz de composición.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

use std::sync::Arc;

use adapters::paths::Paths;
use application::invocation::PendingInvocation;
use ports::VersionMemory;

/// La raíz de `desktop`: las rutas de la máquina, la invocación pendiente y la memoria de la versión.
pub struct DesktopRoot {
    /// Las rutas de configuración, estado y datos.
    pub paths: Paths,
    /// La invocación inicial hasta que la ventana la consume.
    pub pending_invocation: PendingInvocation,
    /// Donde se recuerda la última comprobación de versión.
    pub memory: Arc<dyn VersionMemory + Send + Sync>,
}
