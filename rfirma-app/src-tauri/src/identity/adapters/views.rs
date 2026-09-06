//! Los tipos de identidad que cruzan a la ventana principal (ADR-0011).

use serde::Serialize;

use crate::identity::domain::secret::StoreSecret;
use crate::identity::domain::store::StoreClass;

use crate::signing::adapters::views::StatusView;

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

#[cfg(test)]
mod tests;
