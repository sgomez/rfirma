//! Los tipos del escritorio que cruzan a la ventana principal (ADR-0011).

use serde::Serialize;

/// Estado del manejador de enlaces afirma:// en el sistema.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlHandlersView {
    /// Si el entorno permite consultar manejadores de protocolo.
    pub available: bool,
    /// Manejadores registrados en el escritorio.
    pub handlers: Vec<UrlHandlerView>,
    /// Manejador asignado por defecto.
    pub current: Option<String>,
    /// Identificador de escritorio de esta aplicación.
    pub ours: String,
}

/// Manejador registrado para el esquema de protocolo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlHandlerView {
    /// Identificador de la aplicación en el escritorio.
    pub id: String,
    /// Nombre visible de la aplicación.
    pub name: String,
}

/// Notificación de nueva versión disponible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewVersionView {
    /// Versión publicada.
    pub version: String,
}
