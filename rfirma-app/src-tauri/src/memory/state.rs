//! Estado acumulado por la aplicación persistido entre sesiones (ADR-0010).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pkcs11::CertificateRef;

use super::recents::Recents;

/// Estado acumulado por la aplicación entre ejecuciones (ADR-0010).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Bandeja de documentos recientes.
    pub recents: Recents,
    /// Configuración global de firma visible recordada.
    pub visible_signature: Option<VisibleSignatureMemory>,
    /// Referencia al último certificado utilizado.
    pub certificate: Option<CertificateRef>,
    /// Última carpeta abierta fuera del sandbox (ADR-0011).
    pub last_open_folder: Option<PathBuf>,
    /// Última comprobación de versión realizada.
    pub version_check: Option<VersionCheck>,
}

/// Registro de la última comprobación de actualización de versión.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionCheck {
    /// Cuándo se preguntó, en segundos desde el epoch.
    pub checked_at: u64,
    /// La versión que anunció GitHub, tal y como se leyó.
    pub announced: String,
}

/// Configuración global recordada para firma visible.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisibleSignatureMemory {
    /// El interruptor: si se estampa recuadro.
    pub enabled: bool,
    /// Si la rúbrica va dentro del recuadro. Es la quinta casilla.
    pub rubric: bool,
    /// Las cuatro casillas de texto.
    pub fields: RememberedFields,
    /// El motivo escrito. Vacío es «sin motivo».
    pub reason: String,
    /// El tamaño del recuadro, en espacio de usuario PDF.
    pub size: BoxSize,
}

/// Casillas de texto visibles seleccionadas para la firma.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RememberedFields {
    pub signer_name: bool,
    pub issuer: bool,
    pub signed_at: bool,
    pub reason: bool,
}

/// Dimensiones del recuadro de firma en puntos de espacio de usuario.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoxSize {
    pub width: f64,
    pub height: f64,
}

impl State {
    /// Olvida la actividad acumulada conservando la caché de versión.
    pub fn forget_everything(&mut self) {
        let version_check = self.version_check.take();
        *self = Self::default();
        self.version_check = version_check;
    }

    /// Si no hay nada que recordar.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[cfg(test)]
mod tests;
