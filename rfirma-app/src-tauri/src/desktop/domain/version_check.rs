//! La última comprobación de versión publicada, tal como se recuerda entre sesiones (ADR-0015).

use serde::{Deserialize, Serialize};

/// Registro de la última comprobación de actualización de versión.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionCheck {
    /// Cuándo se preguntó, en segundos desde el epoch.
    pub checked_at: u64,
    /// La versión que anunció GitHub, tal y como se leyó.
    pub announced: String,
}
