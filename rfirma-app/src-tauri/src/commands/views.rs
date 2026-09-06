//! Tipos de salida que cruzan hacia la ventana principal y sus conversiones (ADR-0011).

use serde::{Deserialize, Serialize};

use crate::memory::{Badge, Theme};
use crate::pkcs11::{CertificateStatus, StoreClass, StoreSecret};
use crate::signing::PageSet;

pub use super::failure::Failure;

/// Estado de un certificado tal como cruza a la ventana.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StatusView {
    #[serde(rename_all = "camelCase")]
    Valid {
        not_after: u64,
    },
    #[serde(rename_all = "camelCase")]
    Expired {
        not_after: u64,
    },
    #[serde(rename_all = "camelCase")]
    NotYetValid {
        not_before: u64,
    },
    Revoked {
        reason: String,
    },
    Unreadable {
        detail: String,
    },
}

impl From<CertificateStatus> for StatusView {
    fn from(status: CertificateStatus) -> Self {
        match status {
            CertificateStatus::Valid { not_after } => Self::Valid { not_after },
            CertificateStatus::Expired { not_after } => Self::Expired { not_after },
            CertificateStatus::NotYetValid { not_before } => Self::NotYetValid { not_before },
            CertificateStatus::Revoked { reason } => Self::Revoked { reason },
            CertificateStatus::Unreadable { detail } => Self::Unreadable { detail },
        }
    }
}

/// Forma de solicitar el secreto al almacén de claves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SecretView {
    NotNeeded,
    #[serde(rename_all = "camelCase")]
    TypedOnScreen {
        /// Intentos restantes.
        attempts_left: Option<u32>,
    },
    TypedOnTheReaderKeypad,
}

impl From<StoreSecret> for SecretView {
    fn from(secret: StoreSecret) -> Self {
        match secret {
            StoreSecret::NotNeeded => Self::NotNeeded,
            StoreSecret::TypedOnScreen { attempts_left } => Self::TypedOnScreen { attempts_left },
            StoreSecret::TypedOnTheReaderKeypad => Self::TypedOnTheReaderKeypad,
        }
    }
}

/// Nombre en inglés de una clase de almacén para su traducción en la ventana.
pub fn store_name(class: StoreClass) -> &'static str {
    match class {
        StoreClass::Card => "card",
        StoreClass::Firefox => "firefox",
        StoreClass::Chrome => "chrome",
        StoreClass::Nssdb => "nssdb",
        StoreClass::Installed => "installed",
    }
}

/// Certificado para mostrar en la lista y volver a seleccionarlo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateView {
    /// Asa opaca asignada al listar.
    pub id: String,
    pub label: String,
    pub holder_name: String,
    pub id_number: String,
    pub issuer: String,
    /// Clase de almacén del certificado.
    pub store: String,
    pub status: StatusView,
    /// Si fue el certificado usado en la última firma.
    pub remembered: bool,
}

/// Destino previsto para el documento firmado (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationView {
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Nombre del fichero firmado resultante.
    pub name: Option<String>,
    /// Si la carpeta de destino tiene permisos de escritura.
    pub writable: bool,
}

/// Documento firmado resultante (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedDocumentView {
    /// Nombre del fichero resultante.
    pub name: String,
    /// Nombre de la carpeta de destino.
    pub folder: String,
    /// Tamaño en bytes del fichero escrito.
    pub size_bytes: u64,
}

/// Documento abierto para su visualización o firma (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocumentView {
    /// Identificador opaco asignado al documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Ruta en el anfitrión si está disponible.
    pub path: Option<String>,
}

/// Resultado de soltar ficheros sobre la ventana (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedDocumentView {
    /// Documento abierto en el visor.
    pub document: Option<OpenedDocumentView>,
    /// Documentos adicionales incorporados a recientes.
    pub also_entering: Vec<OpenedDocumentView>,
    /// Motivo del fallo si no se pudo abrir ningún documento.
    pub failure: Option<Failure>,
    /// Número de ficheros descartados que no se incorporaron.
    pub discarded: usize,
}

/// Posición y páginas del recuadro de firma visible.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementView {
    /// Coordenadas del recuadro en espacio de usuario PDF: [x0, y0, x1, y1].
    pub rect: [f64; 4],
    /// Páginas en las que estampar la firma.
    pub pages: PageSet,
}

/// Entrada de la lista de documentos recientes (ADR-0011).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentDocumentView {
    /// Identificador opaco del documento.
    pub id: String,
    /// Nombre del fichero.
    pub name: String,
    /// Insignia o estado del documento.
    pub badge: Badge,
    /// Fecha de modificación en segundos Unix.
    pub modified: Option<u64>,
    /// Fecha de último uso en segundos Unix.
    pub last_used: u64,
    /// Si el fichero sigue existiendo en disco.
    pub available: bool,
    /// Posición del recuadro guardada para este documento.
    pub placement: Option<PlacementView>,
}

/// Configuración de la aplicación visible para la ventana (ADR-0011).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationView {
    /// Idioma seleccionado.
    pub language: String,
    /// Nombre de la carpeta de destino.
    pub destination: String,
    /// Si se recuerda la última configuración de firma visible.
    pub remember_visible_signature: bool,
    /// Si se conserva el historial de actividad reciente.
    pub remember_activity: bool,
    /// Si se notifica la disponibilidad de nuevas versiones.
    pub notify_new_version: bool,
    /// Tema visual de la ventana.
    pub theme: Theme,
    /// Si la plataforma permite guardar junto al original.
    #[serde(default)]
    pub offers_the_original_folder: bool,
    /// Si se ha mostrado ya el aviso de confianza inicial.
    pub trust_notice_seen: bool,
    /// Si se debe consultar por el manejador de enlaces del protocolo.
    pub ask_about_url_handler: bool,
}

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

#[cfg(test)]
mod tests;
