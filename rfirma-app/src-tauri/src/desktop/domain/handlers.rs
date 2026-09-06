//! Quién atiende `afirma://` en el escritorio, tal como lo decide el caso de uso.

/// Estado del manejador de enlaces `afirma://` en el sistema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlHandlers {
    /// Si el entorno permite consultar manejadores de protocolo.
    pub available: bool,
    /// Manejadores registrados en el escritorio.
    pub handlers: Vec<UrlHandler>,
    /// Manejador asignado por defecto.
    pub current: Option<String>,
    /// Identificador de escritorio de esta aplicación.
    pub ours: String,
}

/// Manejador registrado para el esquema de protocolo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UrlHandler {
    /// Identificador de la aplicación en el escritorio.
    pub id: String,
    /// Nombre visible de la aplicación.
    pub name: String,
}
