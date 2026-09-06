//! Los tipos de firma local que cruzan a la ventana principal (ADR-0011).

use serde::{Deserialize, Serialize};

use crate::identity::domain::certificate::CertificateStatus;
use crate::signing::application::configuration::Preferences;
use crate::signing::application::configuration_memory::Theme;
use crate::signing::domain::{PageSet, VisibleBox};

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

/// Posición y páginas del recuadro de firma visible.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementView {
    /// Coordenadas del recuadro en espacio de usuario PDF: [x0, y0, x1, y1].
    pub rect: [f64; 4],
    /// Páginas en las que estampar la firma.
    pub pages: PageSet,
}

impl From<VisibleBox> for PlacementView {
    fn from(placed: VisibleBox) -> Self {
        Self {
            rect: placed.rect,
            pages: placed.pages,
        }
    }
}

impl From<PlacementView> for VisibleBox {
    fn from(view: PlacementView) -> Self {
        Self {
            rect: view.rect,
            pages: view.pages,
        }
    }
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

impl From<Preferences> for ConfigurationView {
    fn from(preferences: Preferences) -> Self {
        Self {
            language: preferences.language,
            destination: preferences.destination,
            remember_visible_signature: preferences.remember_visible_signature,
            remember_activity: preferences.remember_activity,
            notify_new_version: preferences.notify_new_version,
            theme: preferences.theme,
            offers_the_original_folder: preferences.offers_the_original_folder,
            trust_notice_seen: preferences.trust_notice_seen,
            ask_about_url_handler: preferences.ask_about_url_handler,
        }
    }
}

impl From<ConfigurationView> for Preferences {
    fn from(view: ConfigurationView) -> Self {
        Self {
            language: view.language,
            destination: view.destination,
            remember_visible_signature: view.remember_visible_signature,
            remember_activity: view.remember_activity,
            notify_new_version: view.notify_new_version,
            theme: view.theme,
            offers_the_original_folder: view.offers_the_original_folder,
            trust_notice_seen: view.trust_notice_seen,
            ask_about_url_handler: view.ask_about_url_handler,
        }
    }
}

#[cfg(test)]
mod tests;
