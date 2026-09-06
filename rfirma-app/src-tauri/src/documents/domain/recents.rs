//! Lo que un documento reciente dice de sí mismo sin tocar el disco.

use serde::{Deserialize, Serialize};

/// Estado de firma persistido en caché para un documento reciente.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Badge {
    /// Documento con al menos una firma.
    Signed,
    /// Documento sin firmas.
    Unsigned,
}
